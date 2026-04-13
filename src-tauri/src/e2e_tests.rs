//! End-to-end integration test for the JSONL → Brain → DB pipeline.
//!
//! Simulates the full lifecycle that `start_v2_watcher` in lib.rs drives:
//! 1. JSONL message arrives → Brain creates context + stores raw prompt
//! 2. Git signal triggers status transitions (running → done → parked → running)
//! 3. Accumulate prompts → distillation fires → intent created in DB
//! 4. Compression cycle archives stale intents
//! 5. All v2 Tauri API queries return consistent data

use crate::brain::{Brain, JsonlMessage};
use crate::db::Database;
use crate::llm::mock::MockLlmProvider;
use crate::llm::{LlmConfig, LlmRouter};
use std::sync::Arc;

fn setup() -> (Brain, Arc<MockLlmProvider>) {
    let db = Arc::new(Database::in_memory().unwrap());
    let mock = Arc::new(MockLlmProvider::new());
    let router = Arc::new(LlmRouter::new(mock.clone(), LlmConfig::default()));
    let brain = Brain::new(db, router);
    (brain, mock)
}

fn make_msg(project_dir: &str, role: &str, content: &str) -> JsonlMessage {
    JsonlMessage {
        project_hash: format!("hash-{}", project_dir.replace('/', "-")),
        session_id: "session-e2e".to_string(),
        project_dir: project_dir.to_string(),
        display_name: project_dir.split('/').last().unwrap_or("proj").to_string(),
        message_id: uuid::Uuid::now_v7().to_string(),
        role: role.to_string(),
        content: content.to_string(),
    }
}

/// Full lifecycle: ingest → status transitions → distillation → query APIs
#[test]
fn test_full_pipeline_lifecycle() {
    let (brain, mock) = setup();
    let dir = "/home/user/my-app";

    // ── Phase 1: First JSONL message creates context ──
    let msg1 = make_msg(dir, "user", "Fix the authentication bug in login.rs");
    let (ctx_id, prompt_id1) = brain.handle_raw_prompt(msg1).unwrap();
    assert!(!ctx_id.is_empty());
    assert!(!prompt_id1.is_empty());

    // Context should appear in list with status "running"
    let contexts = brain.get_contexts().unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].status, "running");
    assert_eq!(contexts[0].project_dir, dir);
    assert_eq!(contexts[0].name, "my-app");

    // Detail should have no intent yet
    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert!(detail.current_intent.is_none());

    // ── Phase 2: More messages accumulate (simulating ongoing session) ──
    let msg2 = make_msg(dir, "assistant", "I found the bug in auth middleware.");
    let (ctx_id2, _) = brain.handle_raw_prompt(msg2).unwrap();
    assert_eq!(ctx_id, ctx_id2, "Same project_dir should reuse context");

    let msg3 = make_msg(dir, "user", "Great, please fix it and add a test");
    let (ctx_id3, _) = brain.handle_raw_prompt(msg3).unwrap();
    assert_eq!(ctx_id, ctx_id3);

    // Still 1 context
    assert_eq!(brain.get_contexts().unwrap().len(), 1);

    // ── Phase 3: Distillation fires (3 prompts accumulated) ──
    mock.enqueue_distill_response("Fixing authentication bug in session token validation", false);
    let (intent, direction_change) = brain.maybe_distill(&ctx_id).unwrap();
    assert!(intent.is_some(), "Should distill after 3 prompts");
    assert!(!direction_change);

    let intent = intent.unwrap();
    assert_eq!(intent.tier, "narrative");
    assert_eq!(intent.content, "Fixing authentication bug in session token validation");
    assert_eq!(intent.source, "auto");

    // Detail now shows current intent
    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert!(detail.current_intent.is_some());
    assert_eq!(
        detail.current_intent.as_ref().unwrap().content,
        "Fixing authentication bug in session token validation"
    );

    // Timeline shows the intent
    let timeline = brain.get_intent_timeline(&ctx_id, 10, None).unwrap();
    assert_eq!(timeline.intents.len(), 1);
    assert!(!timeline.has_more);
    assert_eq!(timeline.hidden_count, 0);

    // ── Phase 4: Git signals drive status transitions ──

    // running → done (commit/push detected)
    let (_, status) = brain.handle_git_signal(dir, "git_commit_or_push", None).unwrap();
    assert_eq!(status, "done");

    // done → parked (inactivity)
    let (_, status) = brain.handle_git_signal(dir, "no_activity_30min", None).unwrap();
    assert_eq!(status, "parked");

    // parked → running (new session detected)
    let (_, status) = brain.handle_git_signal(dir, "new_session_detected", None).unwrap();
    assert_eq!(status, "running");

    // Verify status via API
    let contexts = brain.get_contexts().unwrap();
    assert_eq!(contexts[0].status, "running");

    // ── Phase 5: Second distillation with direction change ──
    for i in 0..3 {
        let msg = make_msg(dir, "user", &format!("Now switching to REST API refactor, step {i}"));
        brain.handle_raw_prompt(msg).unwrap();
    }

    mock.enqueue_distill_response("Refactoring from GraphQL to REST API endpoints", true);
    let (intent2, direction_change) = brain.maybe_distill(&ctx_id).unwrap();
    assert!(intent2.is_some());
    assert!(direction_change, "Should detect direction change");

    // Timeline now has 2 intents
    let timeline = brain.get_intent_timeline(&ctx_id, 10, None).unwrap();
    assert_eq!(timeline.intents.len(), 2);

    // ── Phase 6: Manual intent submission ──
    let manual_id = brain
        .submit_manual_intent(Some(&ctx_id), "User override: actually doing GraphQL migration")
        .unwrap();
    assert!(!manual_id.is_empty());

    let timeline = brain.get_intent_timeline(&ctx_id, 10, None).unwrap();
    assert_eq!(timeline.intents.len(), 3);

    // ── Phase 7: Intent correction ──
    let correction_id = brain
        .correct_intent(&manual_id, "Corrected: REST to gRPC migration")
        .unwrap();
    assert_ne!(correction_id, manual_id);

    let timeline = brain.get_intent_timeline(&ctx_id, 10, None).unwrap();
    assert_eq!(timeline.intents.len(), 4);

    // ── Phase 8: Manual status override with cooldown ──
    let ok = brain.override_status(&ctx_id, "stuck").unwrap();
    assert!(ok);

    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert_eq!(detail.context.status, "stuck");

    // Cooldown blocks second override
    let ok = brain.override_status(&ctx_id, "running").unwrap();
    assert!(!ok, "Should be blocked by 15-min cooldown");

    // Status should still be stuck
    let detail = brain.get_context_detail(&ctx_id).unwrap();
    assert_eq!(detail.context.status, "stuck");

    // ── Phase 9: LLM status check ──
    let llm_status = brain.llm_status();
    assert!(llm_status.get("mode").is_some());
    assert!(llm_status.get("ollama_ok").is_some());
}

/// Multiple projects running concurrently, up to the max of 4
#[test]
fn test_multi_project_pipeline() {
    let (brain, mock) = setup();

    let projects = [
        ("/home/user/frontend", "Fixing React component rendering"),
        ("/home/user/backend", "Adding REST API endpoint for users"),
        ("/home/user/infra", "Updating Terraform VPC config"),
        ("/home/user/docs", "Writing API documentation"),
    ];

    // Create 4 contexts
    let mut ctx_ids = Vec::new();
    for (dir, content) in &projects {
        let msg = make_msg(dir, "user", content);
        let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();
        ctx_ids.push(ctx_id);
    }

    assert_eq!(brain.get_contexts().unwrap().len(), 4);

    // 5th project should fail
    let msg = make_msg("/home/user/overflow", "user", "This should fail");
    assert!(brain.handle_raw_prompt(msg).is_err());

    // Each project can independently receive messages and transition
    for (i, (dir, _)) in projects.iter().enumerate() {
        // Add more messages
        for j in 0..2 {
            let msg = make_msg(dir, "user", &format!("Project {i} message {j}"));
            brain.handle_raw_prompt(msg).unwrap();
        }

        // Distill each project
        mock.enqueue_distill_response(&format!("Working on project {i}"), false);
        let (intent, _) = brain.maybe_distill(&ctx_ids[i]).unwrap();
        assert!(intent.is_some());

        // Transition to done
        brain.handle_git_signal(dir, "git_commit_or_push", None).unwrap();
    }

    // All should be "done"
    let contexts = brain.get_contexts().unwrap();
    for ctx in &contexts {
        assert_eq!(ctx.status, "done");
    }

    // Each has 1 intent
    for ctx_id in &ctx_ids {
        let timeline = brain.get_intent_timeline(ctx_id, 10, None).unwrap();
        assert_eq!(timeline.intents.len(), 1);
    }
}

/// Compression cycle integrates with the full pipeline
#[test]
fn test_pipeline_with_compression() {
    let (brain, mock) = setup();
    let dir = "/home/user/project";

    // Create context
    let msg = make_msg(dir, "user", "Start");
    let (ctx_id, _) = brain.handle_raw_prompt(msg).unwrap();

    // Insert old intents that qualify for compression (>4h old)
    let old_time = (chrono::Utc::now() - chrono::Duration::hours(5)).to_rfc3339();
    {
        let conn = brain.db.conn();
        for i in 0..3 {
            conn.execute(
                "INSERT INTO intents_v2 (id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from)
                 VALUES (?1, ?2, 'narrative', ?3, 'auto', ?4, 0, NULL, NULL)",
                rusqlite::params![
                    format!("old-intent-{i}"),
                    ctx_id,
                    format!("Old work item {i}"),
                    old_time,
                ],
            ).unwrap();
        }
    }

    // Also add a recent intent (should NOT be compressed)
    brain.submit_manual_intent(Some(&ctx_id), "Current work in progress").unwrap();

    // Run compression
    mock.enqueue_compress_response("Summary of old work items 0-2");
    let compressed_count = brain.run_compression_cycle().unwrap();
    assert_eq!(compressed_count, 1, "Should create 1 compressed intent from 3 old ones");

    // Timeline should show: compressed intent + recent manual intent
    let timeline = brain.get_intent_timeline(&ctx_id, 10, None).unwrap();
    let non_archived: Vec<_> = timeline.intents.iter().filter(|i| !i.archived).collect();
    assert!(non_archived.len() >= 2, "Should have compressed + recent intents");
    assert_eq!(timeline.hidden_count, 3, "3 old intents should be archived");

    // Expand the compressed intent
    let compressed = timeline
        .intents
        .iter()
        .find(|i| i.compressed_from.is_some())
        .expect("Should have a compressed intent");
    let sources = brain.expand_compressed_intent(&compressed.id).unwrap();
    assert_eq!(sources.len(), 3);
}
