pub mod db;
pub mod intent;
pub mod watcher;
pub mod capture;
pub mod events;
pub mod commands;

use std::sync::Arc;
use tauri::Manager;
use watcher::{AgentWatcher, ClaudeCodeWatcher, SessionStatus};
use intent::SimpleRuleExtractor;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Initialize database
            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).ok();
            let db_path = app_data_dir.join("stash.db");
            let database = db::Database::new(&db_path).expect("failed to initialize database");
            let db = Arc::new(database);
            app.manage(db.clone());

            // Initialize event aggregator
            let aggregator = Arc::new(events::EventAggregator::new());
            app.manage(aggregator.clone());

            // Set up tray icon click handler
            if let Some(tray) = app.tray_by_id("main") {
                let handle = app_handle.clone();
                tray.on_tray_icon_event(move |_tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        if let Some(window) = handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
            }

            // Start auto-capture background loop
            let capture_db = db.clone();
            let capture_agg = aggregator.clone();
            let capture_handle = app_handle.clone();
            std::thread::spawn(move || {
                auto_capture_loop(capture_db, capture_agg, capture_handle);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::task_create,
            commands::task_list,
            commands::task_get_card,
            commands::task_switch,
            commands::task_park,
            commands::save_resume_note,
            commands::refine_intent,
            commands::mark_drift,
            commands::create_manual_branch,
            commands::update_agent_branch,
            commands::get_briefing,
            commands::get_graph_data,
            commands::park_all_tasks,
            commands::start_review,
            commands::end_review,
            commands::open_external,
            commands::set_task_dependency,
            commands::query_review_logs,
            commands::get_unreviewed_branch_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Background loop: polls Claude Code sessions every 5s, auto-creates tasks/intents/branches.
fn auto_capture_loop(
    db: Arc<db::Database>,
    aggregator: Arc<events::EventAggregator>,
    app_handle: tauri::AppHandle,
) {
    let watcher = ClaudeCodeWatcher::new();
    let extractor = SimpleRuleExtractor::new();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));

        let sessions = watcher.detect_sessions();
        for session in sessions {
            if let Err(e) = process_session(&db, &aggregator, &extractor, &session, &app_handle) {
                tracing::warn!("Auto-capture error for session {}: {}", session.session_id, e);
            }
        }
    }
}

fn process_session(
    db: &db::Database,
    aggregator: &events::EventAggregator,
    extractor: &SimpleRuleExtractor,
    session: &watcher::DetectedSession,
    app_handle: &tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;

    // Skip sessions with no user messages
    if session.user_messages.is_empty() {
        return Ok(());
    }

    // Try to find or create a task for this session's working directory
    let task_name = session
        .working_dir
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(&session.session_id)
        .to_string();

    // Check if we already have a task matching this project name
    let tasks = db.task_list(None)?;
    let existing_task = tasks.iter().find(|t| t.name == task_name);

    let task_id = if let Some(t) = existing_task {
        t.id.clone()
    } else {
        // Auto-create task from first user message
        let first_msg = &session.user_messages[0].content;
        let intent = extractor
            .extract_intent(first_msg)
            .unwrap_or_else(|| first_msg.chars().take(120).collect());

        let task = db.task_create(&task_name)?;
        db.intent_create(&task.id, &intent, "auto_inferred", Some("auto-captured from Claude Code session"))?;
        tracing::info!("Auto-created task '{}' with intent: {}", task_name, intent);
        task.id
    };

    // Check if we have a branch for this session
    let branches = db.branch_list(&task_id)?;
    let session_branch = branches
        .iter()
        .find(|b| b.output_ref.as_deref() == Some(&session.session_id));

    let branch_id = if let Some(b) = session_branch {
        // Update branch status
        let new_status = match session.status {
            SessionStatus::Running => "running",
            SessionStatus::Idle => "running",
            SessionStatus::Completed => "completed",
            SessionStatus::Error => "error",
        };
        if b.status != new_status {
            db.branch_update(&b.id, Some(new_status), None, None)?;
            aggregator.notify_event();
        }
        b.id.clone()
    } else {
        // Create branch for this session
        let current_intent = db.intent_get_current(&task_id)?;
        let intent_id = current_intent
            .map(|i| i.id)
            .unwrap_or_default();

        if intent_id.is_empty() {
            return Ok(());
        }

        let branch = db.branch_create(
            &task_id,
            "claude-code",
            "#7c3aed", // purple for Claude
            &intent_id,
            "auto",
        )?;
        // Store session_id in output_ref for tracking
        db.branch_update(&branch.id, None, None, Some(&session.session_id))?;
        aggregator.notify_event();
        tracing::info!("Auto-created branch for session {}", session.session_id);
        branch.id
    };

    // Process new user messages → refine intent if they indicate a direction change
    for msg in &session.user_messages {
        if let Some(extracted) = extractor.extract_intent(&msg.content) {
            // Check if this is meaningfully different from current intent
            if let Ok(Some(current)) = db.intent_get_current(&task_id) {
                if extracted != current.statement && extracted.chars().count() > 10 {
                    db.intent_create(&task_id, &extracted, "auto_inferred", Some(&msg.content))?;
                    tracing::info!("Auto-refined intent for task {}: {}", task_name, extracted);
                }
            }
        }
    }

    // Emit event to frontend for real-time update
    let _ = app_handle.emit("stash://capture-update", &task_id);

    Ok(())
}
