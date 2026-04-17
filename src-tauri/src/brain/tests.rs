use super::*;
use crate::db::Database;
use crate::llm::mock::MockLlmProvider;
use crate::llm::{LlmConfig, LlmRouter};
use std::sync::Arc;

fn setup() -> (Brain, Arc<MockLlmProvider>) {
    let db = Arc::new(Database::in_memory().unwrap());
    let mock_provider = Arc::new(MockLlmProvider::new());
    let router = Arc::new(LlmRouter::new(
        mock_provider.clone(),
        LlmConfig::default(),
    ));
    let brain = Brain::new(db, router);
    (brain, mock_provider)
}

fn make_message(project_hash: &str, project_dir: &str, content: &str) -> JsonlMessage {
    JsonlMessage {
        project_hash: project_hash.to_string(),
        session_id: "session-1".to_string(),
        project_dir: project_dir.to_string(),
        display_name: project_dir
            .split('/')
            .next_back()
            .unwrap_or("project")
            .to_string(),
        message_id: uuid::Uuid::now_v7().to_string(),
        role: "user".to_string(),
        content: content.to_string(),
    }
}

// ── handle_raw_prompt tests ──

#[test]
fn test_handle_raw_prompt_creates_context_and_stores_prompt() {
    let (brain, _mock) = setup();
    let msg = make_message("hash1", "/home/user/project1", "Fix the login bug");

    let (context_id, prompt_id) = brain.handle_raw_prompt(msg).unwrap();

    assert!(!context_id.is_empty());
    assert!(!prompt_id.is_empty());

    // Verify context was created
    let contexts = brain.get_contexts().unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].project_dir, "/home/user/project1");
}

#[test]
fn test_handle_raw_prompt_reuses_existing_context() {
    let (brain, _mock) = setup();

    let msg1 = make_message("hash1", "/home/user/project1", "First message");
    let (ctx1, _) = brain.handle_raw_prompt(msg1).unwrap();

    let msg2 = make_message("hash1", "/home/user/project1", "Second message");
    let (ctx2, _) = brain.handle_raw_prompt(msg2).unwrap();

    // Same context
    assert_eq!(ctx1, ctx2);

    // Only 1 context
    let contexts = brain.get_contexts().unwrap();
    assert_eq!(contexts.len(), 1);
}

#[test]
fn test_handle_raw_prompt_enforces_max_4_contexts() {
    let (brain, _mock) = setup();

    // Create 4 contexts
    for i in 0..4 {
        let msg = make_message(
            &format!("hash{i}"),
            &format!("/home/user/project{i}"),
            &format!("Message for project {i}"),
        );
        brain.handle_raw_prompt(msg).unwrap();
    }

    // 5th context should fail
    let msg = make_message("hash4", "/home/user/project4", "Too many projects");
    let result = brain.handle_raw_prompt(msg);

    assert!(result.is_err());
    match result.unwrap_err() {
        BrainError::MaxContextsReached => {} // expected
        other => panic!("Expected MaxContextsReached, got: {other}"),
    }
}

#[test]
fn test_handle_raw_prompt_existing_context_does_not_count_against_limit() {
    let (brain, _mock) = setup();

    // Create 4 contexts
    for i in 0..4 {
        let msg = make_message(
            &format!("hash{i}"),
            &format!("/home/user/project{i}"),
            &format!("Message {i}"),
        );
        brain.handle_raw_prompt(msg).unwrap();
    }

    // Sending to existing context should succeed even at max
    let msg = make_message("hash0", "/home/user/project0", "Another message");
    let result = brain.handle_raw_prompt(msg);
    assert!(result.is_ok());
}

// ── Status state machine tests ──

#[test]
fn test_status_transitions_via_git_signal() {
    let (brain, _mock) = setup();

    // Create a context
    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // Initial status is running
    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert_eq!(detail.context.status, "running");

    // running → done via git_commit_or_push
    let (_, status) = brain
        .handle_git_signal("/home/user/proj", "git_commit_or_push", None)
        .unwrap();
    assert_eq!(status, "done");

    // done → parked via no_activity_30min
    let (_, status) = brain
        .handle_git_signal("/home/user/proj", "no_activity_30min", None)
        .unwrap();
    assert_eq!(status, "parked");

    // parked → running via new_session_detected
    let (_, status) = brain
        .handle_git_signal("/home/user/proj", "new_session_detected", None)
        .unwrap();
    assert_eq!(status, "running");

    // running → stuck via error_pattern_10min
    let (_, status) = brain
        .handle_git_signal("/home/user/proj", "error_pattern_10min", None)
        .unwrap();
    assert_eq!(status, "stuck");

    // stuck → running via new_non_error_prompts
    let (_, status) = brain
        .handle_git_signal("/home/user/proj", "new_non_error_prompts", None)
        .unwrap();
    assert_eq!(status, "running");
}

#[test]
fn test_invalid_signal_does_not_transition() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    brain.handle_raw_prompt(msg).unwrap();

    // running + new_session_detected → no transition (already running via parked path only)
    let (_, status) = brain
        .handle_git_signal("/home/user/proj", "new_prompts_detected", None)
        .unwrap();
    // Should remain running (no valid transition from running via new_prompts_detected)
    assert_eq!(status, "running");
}

#[test]
fn test_git_signal_unknown_project_fails() {
    let (brain, _mock) = setup();

    let result = brain.handle_git_signal("/unknown/dir", "git_commit_or_push", None);
    assert!(result.is_err());
    match result.unwrap_err() {
        BrainError::ContextNotFound(_) => {} // expected
        other => panic!("Expected ContextNotFound, got: {other}"),
    }
}

// ── maybe_distill tests ──

#[test]
fn test_maybe_distill_triggers_at_threshold() {
    let (brain, mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // Add enough prompts to reach threshold (we already have 1)
    for i in 1..3 {
        let msg = make_message(
            "hash1",
            "/home/user/proj",
            &format!("Prompt {i}"),
        );
        brain.handle_raw_prompt(msg).unwrap();
    }

    // Now we have 3 pending prompts, enqueue mock response
    mock.enqueue_distill_response("Fixing authentication bugs in login flow", false);

    let (intent, direction_change) = brain.maybe_distill(&ctx_id).unwrap();

    assert!(intent.is_some());
    let intent = intent.unwrap();
    assert_eq!(intent.tier, "narrative");
    assert_eq!(
        intent.content,
        "Fixing authentication bugs in login flow"
    );
    assert!(!direction_change);
}

#[test]
fn test_maybe_distill_below_threshold_returns_none() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Only one prompt");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    let (intent, direction_change) = brain.maybe_distill(&ctx_id).unwrap();

    assert!(intent.is_none());
    assert!(!direction_change);
}

#[test]
fn test_maybe_distill_detects_direction_change() {
    let (brain, mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    for i in 1..3 {
        let msg = make_message("hash1", "/home/user/proj", &format!("Prompt {i}"));
        brain.handle_raw_prompt(msg).unwrap();
    }

    mock.enqueue_distill_response("Switching to REST API instead of GraphQL", true);

    let (intent, direction_change) = brain.maybe_distill(&ctx_id).unwrap();

    assert!(intent.is_some());
    assert!(direction_change);
}

#[test]
fn test_maybe_distill_marks_prompts_consumed() {
    let (brain, mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    for i in 1..3 {
        let msg = make_message("hash1", "/home/user/proj", &format!("Prompt {i}"));
        brain.handle_raw_prompt(msg).unwrap();
    }

    mock.enqueue_distill_response("Working on auth", false);

    brain.maybe_distill(&ctx_id).unwrap();

    // After distillation, pending prompts should be consumed
    let pending = brain.db.get_pending_prompts(&ctx_id, 10).unwrap();
    assert_eq!(pending.len(), 0);
}

// ── run_compression_cycle tests ──

#[test]
fn test_compression_cycle_compresses_stale_narratives() {
    let (brain, mock) = setup();

    // Create context
    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // Manually insert stale narrative intents (>4h old by using old timestamps)
    // We directly use db to insert intents with old timestamps for testing
    let old_time = (chrono::Utc::now() - chrono::Duration::hours(5)).to_rfc3339();
    {
        let conn = brain.db.conn();
        conn.execute(
            "INSERT INTO intents_v2 (id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from)
             VALUES ('stale-1', ?1, 'narrative', 'Fixed login bug', 'auto', ?2, 0, NULL, NULL)",
            rusqlite::params![ctx_id, old_time],
        ).unwrap();
        conn.execute(
            "INSERT INTO intents_v2 (id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from)
             VALUES ('stale-2', ?1, 'narrative', 'Added auth tests', 'auto', ?2, 0, NULL, NULL)",
            rusqlite::params![ctx_id, old_time],
        ).unwrap();
    }

    mock.enqueue_compress_response("Authentication work: fixed login and added tests");

    let count = brain.run_compression_cycle().unwrap();
    assert_eq!(count, 1);

    // Verify the stale intents are archived
    let stale1 = brain.db.get_intent_v2("stale-1").unwrap();
    assert!(stale1.archived);
    let stale2 = brain.db.get_intent_v2("stale-2").unwrap();
    assert!(stale2.archived);
}

#[test]
fn test_compression_cycle_no_stale_returns_zero() {
    let (brain, _mock) = setup();

    let count = brain.run_compression_cycle().unwrap();
    assert_eq!(count, 0);
}

// ── Manual override tests ──

#[test]
fn test_manual_override_sets_status() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // Override to done
    let success = brain.override_status(&ctx_id, "done").unwrap();
    assert!(success);

    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert_eq!(detail.context.status, "done");
}

#[test]
fn test_manual_override_cooldown() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // First override succeeds
    let success = brain.override_status(&ctx_id, "done").unwrap();
    assert!(success);

    // Second override should fail (cooldown active)
    let success = brain.override_status(&ctx_id, "running").unwrap();
    assert!(!success);

    // Status should still be done
    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert_eq!(detail.context.status, "done");
}

#[test]
fn test_manual_override_invalid_status_fails() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    let result = brain.override_status(&ctx_id, "invalid_status");
    assert!(result.is_err());
    match result.unwrap_err() {
        BrainError::InvalidStatus(_) => {} // expected
        other => panic!("Expected InvalidStatus, got: {other}"),
    }
}

// ── get_intent_timeline tests ──

#[test]
fn test_intent_timeline_pagination() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // Insert 5 intents
    for i in 0..5 {
        brain
            .submit_manual_intent(Some(&ctx_id), &format!("Intent {i}"))
            .unwrap();
    }

    // Get first page (limit 3)
    let timeline = brain.get_intent_timeline(&ctx_id, 3, None).unwrap();
    assert_eq!(timeline.intents.len(), 3);
    assert!(timeline.has_more);

    // Get next page using cursor
    let last_id = &timeline.intents.last().unwrap().id;
    let page2 = brain
        .get_intent_timeline(&ctx_id, 3, Some(last_id))
        .unwrap();
    assert_eq!(page2.intents.len(), 2);
    assert!(!page2.has_more);
}

#[test]
fn test_intent_timeline_hidden_count() {
    let (brain, mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // Create and archive some intents
    let old_time = (chrono::Utc::now() - chrono::Duration::hours(5)).to_rfc3339();
    {
        let conn = brain.db.conn();
        conn.execute(
            "INSERT INTO intents_v2 (id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from)
             VALUES ('arch-1', ?1, 'narrative', 'Archived 1', 'auto', ?2, 0, NULL, NULL)",
            rusqlite::params![ctx_id, old_time],
        ).unwrap();
        conn.execute(
            "INSERT INTO intents_v2 (id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from)
             VALUES ('arch-2', ?1, 'narrative', 'Archived 2', 'auto', ?2, 0, NULL, NULL)",
            rusqlite::params![ctx_id, old_time],
        ).unwrap();
    }

    // Compress to archive them
    mock.enqueue_compress_response("Summary of archived work");
    brain.run_compression_cycle().unwrap();

    let timeline = brain.get_intent_timeline(&ctx_id, 10, None).unwrap();
    assert_eq!(timeline.hidden_count, 2);
}

// ── expand_compressed_intent tests ──

#[test]
fn test_expand_compressed_intent() {
    let (brain, mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // Create stale narratives
    let old_time = (chrono::Utc::now() - chrono::Duration::hours(5)).to_rfc3339();
    {
        let conn = brain.db.conn();
        conn.execute(
            "INSERT INTO intents_v2 (id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from)
             VALUES ('src-1', ?1, 'narrative', 'Source 1', 'auto', ?2, 0, NULL, NULL)",
            rusqlite::params![ctx_id, old_time],
        ).unwrap();
        conn.execute(
            "INSERT INTO intents_v2 (id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from)
             VALUES ('src-2', ?1, 'narrative', 'Source 2', 'auto', ?2, 0, NULL, NULL)",
            rusqlite::params![ctx_id, old_time],
        ).unwrap();
    }

    mock.enqueue_compress_response("Compressed summary");
    brain.run_compression_cycle().unwrap();

    // Find the compressed intent
    let timeline = brain.get_intent_timeline(&ctx_id, 10, None).unwrap();
    let compressed = timeline
        .intents
        .iter()
        .find(|i| i.compressed_from.is_some())
        .expect("Should have a compressed intent");

    let sources = brain.expand_compressed_intent(&compressed.id).unwrap();
    assert_eq!(sources.len(), 2);
}

#[test]
fn test_expand_non_compressed_intent_fails() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    let intent_id = brain
        .submit_manual_intent(Some(&ctx_id), "Regular intent")
        .unwrap();

    let result = brain.expand_compressed_intent(&intent_id);
    assert!(result.is_err());
    match result.unwrap_err() {
        BrainError::NotCompressed => {} // expected
        other => panic!("Expected NotCompressed, got: {other}"),
    }
}

// ── submit_manual_intent tests ──

#[test]
fn test_submit_manual_intent() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    let intent_id = brain
        .submit_manual_intent(Some(&ctx_id), "User typed this intent")
        .unwrap();
    assert!(!intent_id.is_empty());

    let intent = brain.db.get_intent_v2(&intent_id).unwrap();
    assert_eq!(intent.content, "User typed this intent");
    assert_eq!(intent.source, "manual");
    assert_eq!(intent.tier, "narrative");
}

// ── correct_intent tests ──

#[test]
fn test_correct_intent_creates_correction() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    let original_id = brain
        .submit_manual_intent(Some(&ctx_id), "Original intent")
        .unwrap();

    let correction_id = brain
        .correct_intent(&original_id, "Corrected intent text")
        .unwrap();
    assert!(!correction_id.is_empty());
    assert_ne!(correction_id, original_id);

    let correction = brain.db.get_intent_v2(&correction_id).unwrap();
    assert_eq!(correction.content, "Corrected intent text");
    assert_eq!(correction.source, "manual_correction");
    assert_eq!(correction.context_id, ctx_id);
}

// ── get_context_detail tests ──

#[test]
fn test_get_context_detail_with_intent() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    brain
        .submit_manual_intent(Some(&ctx_id), "Working on auth")
        .unwrap();

    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert_eq!(detail.context.id, ctx_id);
    assert!(detail.current_intent.is_some());
    assert_eq!(detail.current_intent.unwrap().content, "Working on auth");
}

#[test]
fn test_get_context_detail_no_intent() {
    let (brain, _mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert!(detail.current_intent.is_none());
}

// ── LLM unavailable tests ──

#[test]
fn test_distill_with_llm_unavailable() {
    let (brain, mock) = setup();

    let msg = make_message("hash1", "/home/user/proj", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    for i in 1..3 {
        let msg = make_message("hash1", "/home/user/proj", &format!("Prompt {i}"));
        brain.handle_raw_prompt(msg).unwrap();
    }

    mock.set_available(false);

    let result = brain.maybe_distill(&ctx_id);
    assert!(result.is_err());
    match result.unwrap_err() {
        BrainError::LlmUnavailable(_) => {} // expected
        other => panic!("Expected LlmUnavailable, got: {other}"),
    }
}
