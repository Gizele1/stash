use std::sync::Arc;
use tauri::State;
use crate::db::Database;
use crate::db::*;
use crate::events::EventAggregator;

type Db<'a> = State<'a, Arc<Database>>;
type Agg<'a> = State<'a, Arc<EventAggregator>>;
type CmdResult<T> = Result<T, String>;

// ── Task commands ──

#[tauri::command]
pub fn task_create(name: String, initial_intent: String, db: Db<'_>) -> CmdResult<TaskCardData> {
    let task = db.task_create(&name)?;
    db.intent_create(&task.id, &initial_intent, "initial", None)?;
    db.get_task_card_data(&task.id)
}

#[tauri::command]
pub fn task_list(status: Option<String>, db: Db<'_>) -> CmdResult<Vec<Task>> {
    db.task_list(status.as_deref())
}

#[tauri::command]
pub fn task_get_card(task_id: String, db: Db<'_>) -> CmdResult<TaskCardData> {
    db.get_task_card_data(&task_id)
}

#[tauri::command]
pub fn task_switch(task_id: String, db: Db<'_>) -> CmdResult<Task> {
    db.task_update_status(&task_id, "active")
}

#[tauri::command]
pub fn task_park(task_id: String, db: Db<'_>) -> CmdResult<Task> {
    db.task_update_status(&task_id, "parked")
}

// ── Intent commands ──

#[tauri::command]
pub fn refine_intent(
    task_id: String,
    statement: String,
    trigger_type: String,
    reason: Option<String>,
    db: Db<'_>,
) -> CmdResult<IntentSnapshot> {
    db.intent_create(&task_id, &statement, &trigger_type, reason.as_deref())
}

// ── Agent branch commands ──

#[tauri::command]
pub fn create_manual_branch(
    task_id: String,
    agent_platform: String,
    platform_color: String,
    db: Db<'_>,
) -> CmdResult<AgentBranch> {
    let current_intent = db.intent_get_current(&task_id)?
        .ok_or_else(|| "Task has no current intent".to_string())?;

    db.branch_create(&task_id, &agent_platform, &platform_color, &current_intent.id, "manual")
}

#[tauri::command]
pub fn update_agent_branch(
    branch_id: String,
    status: Option<String>,
    progress: Option<f64>,
    output_ref: Option<String>,
    db: Db<'_>,
    aggregator: Agg<'_>,
) -> CmdResult<AgentBranch> {
    let branch = db.branch_update(&branch_id, status.as_deref(), progress, output_ref.as_deref())?;
    if status.is_some() {
        aggregator.notify_event();
    }
    Ok(branch)
}

// ── Drift commands ──

#[tauri::command]
pub fn mark_drift(
    branch_id: String,
    summary: String,
    db: Db<'_>,
    aggregator: Agg<'_>,
) -> CmdResult<DriftMarker> {
    let marker = db.drift_create(&branch_id, &summary)?;
    aggregator.notify_event();
    Ok(marker)
}

// ── Resume notes ──

#[tauri::command]
pub fn save_resume_note(
    task_id: String,
    content: String,
    source: String,
    db: Db<'_>,
) -> CmdResult<ResumeNote> {
    db.resume_note_upsert(&task_id, &content, &source)
}

// ── Briefing ──

#[tauri::command]
pub fn get_briefing(db: Db<'_>, aggregator: Agg<'_>) -> CmdResult<Briefing> {
    let events = db.event_list_unread()?;
    let event_ids: Vec<String> = events.iter().map(|e| e.id.clone()).collect();

    let items_json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string());
    let briefing = db.briefing_save(&items_json, &event_ids)?;
    aggregator.clear_pending();
    Ok(briefing)
}

// ── Graph ──

#[tauri::command]
pub fn get_graph_data(task_id: String, db: Db<'_>) -> CmdResult<GraphData> {
    db.get_graph_data(&task_id)
}

// ── Review ──

#[tauri::command]
pub fn start_review(_task_id: String, _branch_id: String) -> CmdResult<String> {
    Ok(uuid::Uuid::now_v7().to_string())
}

#[tauri::command]
pub fn end_review(
    task_id: String,
    branch_id: String,
    started_at: String,
    duration_seconds: i64,
    outcome: String,
    db: Db<'_>,
) -> CmdResult<ReviewLog> {
    db.review_log_create(&task_id, &branch_id, &started_at, duration_seconds, &outcome)
}

// ── Utility commands ──

#[tauri::command]
pub fn park_all_tasks(db: Db<'_>) -> CmdResult<()> {
    let tasks = db.task_list(Some("active"))?;
    for task in tasks {
        db.task_update_status(&task.id, "parked")?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_external(url: String) -> CmdResult<()> {
    open::that(&url).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_task_dependency(
    from_task_id: String,
    to_task_id: String,
    db: Db<'_>,
) -> CmdResult<()> {
    db.task_set_dependency(&from_task_id, &to_task_id)
}

#[tauri::command]
pub fn query_review_logs(
    task_id: Option<String>,
    date: Option<String>,
    db: Db<'_>,
) -> CmdResult<Vec<ReviewLog>> {
    db.review_log_query(task_id.as_deref(), date.as_deref(), None)
}

#[tauri::command]
pub fn get_unreviewed_branch_count(db: Db<'_>) -> CmdResult<i64> {
    db.get_unreviewed_branch_count()
}
