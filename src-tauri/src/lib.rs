pub mod db;
pub mod intent;
pub mod watcher;
pub mod capture;
pub mod events;
pub mod commands;
pub mod llm;
pub mod brain;
pub mod platform;

use std::sync::Arc;
use tauri::Manager;

use brain::Brain;
use llm::{LlmConfig, LlmRouter, StubLlmProvider};
use watcher::{Watcher, WatcherConfig};

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

            // Initialize LLM router with stub provider
            let llm_provider = Arc::new(StubLlmProvider::new());
            let llm_router = Arc::new(LlmRouter::new(llm_provider, LlmConfig::default()));

            // Initialize Brain (context engine)
            let brain = Arc::new(Brain::new(db.clone(), llm_router));
            app.manage(brain.clone());

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

            // Start v2 watcher background loop
            let watcher_handle = app_handle.clone();
            let watcher_brain = brain.clone();
            let watcher_agg = aggregator.clone();
            std::thread::spawn(move || {
                start_v2_watcher(watcher_handle, watcher_brain, watcher_agg);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // v1 commands (kept for backwards compatibility)
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
            commands::window::open_graph_window,
            // v2 commands (Brain-based context engine)
            commands::get_contexts,
            commands::get_context_detail,
            commands::get_intent_timeline,
            commands::override_status,
            commands::submit_manual_intent,
            commands::correct_intent,
            commands::focus_terminal,
            commands::open_pr_url,
            commands::save_pet_position,
            commands::expand_compressed_intent,
            commands::get_llm_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Start the v2 file-system watcher with notify crate.
/// Routes JSONL messages through Brain for processing, then emits events to frontend.
fn start_v2_watcher(
    app_handle: tauri::AppHandle,
    brain: Arc<Brain>,
    aggregator: Arc<events::EventAggregator>,
) {
    use tauri::Emitter;

    let config = WatcherConfig::default();

    let mut watcher = match Watcher::new(config) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Failed to create watcher: {}", e);
            return;
        }
    };

    let jsonl_handle = app_handle.clone();
    let jsonl_brain = brain.clone();
    let jsonl_agg = aggregator.clone();

    let git_handle = app_handle.clone();
    let git_brain = brain.clone();
    let git_agg = aggregator.clone();

    let join_handle = match watcher.start(
        Box::new(move |_file_path, messages| {
            for msg in &messages {
                tracing::info!(
                    "JSONL: session={} content={}",
                    msg.session_id,
                    &msg.content[..msg.content.len().min(80)]
                );

                // Route through Brain pipeline
                let brain_msg = brain::JsonlMessage {
                    project_hash: msg.project_hash.clone(),
                    session_id: msg.session_id.clone(),
                    project_dir: msg.project_dir.clone(),
                    display_name: msg.display_name.clone(),
                    message_id: uuid::Uuid::now_v7().to_string(),
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                };

                match jsonl_brain.handle_raw_prompt(brain_msg) {
                    Ok((context_id, _prompt_id)) => {
                        tracing::debug!("Brain processed prompt for context {}", context_id);

                        // Drive status machine: new prompts → running
                        let _ = jsonl_brain.handle_git_signal(
                            &msg.project_dir,
                            "new_session_detected",
                            None,
                        );

                        // Try distillation after storing prompt
                        match jsonl_brain.maybe_distill(&context_id) {
                            Ok((Some(intent), _dc)) => {
                                tracing::debug!("Distilled intent for context {}: {}", context_id, intent.content);
                            }
                            Ok((None, _)) => {}
                            Err(e) => {
                                tracing::warn!("Distillation skipped for context {}: {}", context_id, e);
                            }
                        }
                        let _ = jsonl_handle.emit("stash://state-change", serde_json::json!({
                            "event_type": "context_updated",
                            "payload": { "context_id": context_id }
                        }));
                    }
                    Err(e) => {
                        tracing::warn!("Brain failed to process prompt: {}", e);
                    }
                }
            }
            jsonl_agg.notify_event();
            let _ = jsonl_handle.emit("stash://jsonl-messages", &messages);
        }),
        Box::new(move |project_dir, signal_type, metadata| {
            tracing::info!(
                "Git signal: project={} type={} meta={}",
                project_dir,
                signal_type,
                metadata
            );

            // Route through Brain pipeline
            match git_brain.handle_git_signal(project_dir, signal_type, None) {
                Ok((context_id, new_status)) => {
                    tracing::debug!(
                        "Brain processed git signal: context={} status={}",
                        context_id,
                        new_status
                    );
                    let _ = git_handle.emit("stash://state-change", serde_json::json!({
                        "event_type": "status_changed",
                        "payload": { "context_id": context_id, "new_status": new_status }
                    }));
                }
                Err(e) => {
                    // ContextNotFound is expected for projects not yet tracked
                    tracing::debug!("Brain skipped git signal: {}", e);
                }
            }

            git_agg.notify_event();
            let payload = serde_json::json!({
                "project_dir": project_dir,
                "signal_type": signal_type,
                "metadata": metadata,
            });
            let _ = git_handle.emit("stash://git-signal", &payload);
        }),
        Box::new(|old_path, new_path| {
            tracing::info!("JSONL file rotated: {} -> {}", old_path, new_path);
        }),
    ) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to start watcher: {}", e);
            return;
        }
    };

    // Block this thread until the watcher thread finishes
    let _ = join_handle.join();
}
