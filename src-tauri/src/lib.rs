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
            let watcher_agg = aggregator.clone();
            std::thread::spawn(move || {
                start_v2_watcher(watcher_handle, watcher_agg);
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
            commands::window::open_graph_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Start the v2 file-system watcher with notify crate.
/// Forwards JSONL messages and git signals to the frontend via Tauri events.
fn start_v2_watcher(
    app_handle: tauri::AppHandle,
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
    let jsonl_agg = aggregator.clone();

    let git_handle = app_handle.clone();
    let git_agg = aggregator.clone();

    let join_handle = match watcher.start(
        Box::new(move |messages| {
            for msg in &messages {
                tracing::info!(
                    "JSONL: session={} content={}",
                    msg.session_id,
                    &msg.content[..msg.content.len().min(80)]
                );
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
            git_agg.notify_event();
            let payload = serde_json::json!({
                "project_dir": project_dir,
                "signal_type": signal_type,
                "metadata": metadata,
            });
            let _ = git_handle.emit("stash://git-signal", &payload);
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
