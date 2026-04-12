use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextRecord {
    pub id: String,
    pub project_key: String,
    pub project_dir: Option<String>,
    pub name: String,
    pub manual_assignment_required: bool,
    pub status: String,
    pub status_override_until: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawPromptRecord {
    pub id: String,
    pub context_id: String,
    pub session_path: String,
    pub message_id: String,
    pub role: String,
    pub content: String,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentRecord {
    pub id: String,
    pub context_id: String,
    pub tier: String,
    pub content: String,
    pub source: String,
    pub created_at: String,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub compressed_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub llm_mode: String,
    pub local_model: String,
    pub cloud_model: Option<String>,
    pub cloud_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub status: String,
    pub current_intent_id: Option<String>,
    pub created_at: String,
    pub parked_at: Option<String>,
    pub last_active_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentSnapshot {
    pub id: String,
    pub task_id: String,
    pub version: i64,
    pub statement: String,
    pub trigger_type: String,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBranch {
    pub id: String,
    pub task_id: String,
    pub agent_platform: String,
    pub platform_color: String,
    pub forked_from_intent_id: String,
    pub status: String,
    pub progress: Option<f64>,
    pub output_ref: Option<String>,
    pub source_type: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftMarker {
    pub id: String,
    pub branch_id: String,
    pub summary: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeNote {
    pub id: String,
    pub task_id: String,
    pub content: String,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentSnapshot {
    pub id: String,
    pub task_id: String,
    pub git_branch: Option<String>,
    pub git_status: Option<String>,
    pub git_diff_summary: Option<String>,
    pub active_files: Option<String>,
    pub terminal_last_output: Option<String>,
    pub window_focus: Option<String>,
    pub agent_states: Option<String>,
    pub captured_at: String,
    pub completeness: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub branch_id: String,
    pub event_type: String,
    pub summary: Option<String>,
    pub metadata: Option<String>,
    pub created_at: String,
    pub briefing_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewLog {
    pub id: String,
    pub task_id: String,
    pub branch_id: String,
    pub started_at: String,
    pub duration_seconds: i64,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Briefing {
    pub id: String,
    pub generated_at: String,
    pub read_at: Option<String>,
    pub items: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub current_intent_statement: Option<String>,
    pub agent_count: i64,
    pub running_count: i64,
    pub completed_unreviewed_count: i64,
    pub has_drift: bool,
    pub platform_colors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCardData {
    pub task: Task,
    pub current_intent: Option<IntentSnapshot>,
    pub branches: Vec<AgentBranch>,
    pub resume_note: Option<ResumeNote>,
    pub latest_snapshot: Option<EnvironmentSnapshot>,
    pub has_drift: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub intent_nodes: Vec<IntentSnapshot>,
    pub branch_edges: Vec<BranchEdgeData>,
    pub current_intent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchEdgeData {
    pub branch_id: String,
    pub platform: String,
    pub color: String,
    pub forked_from_intent_id: String,
    pub status: String,
    pub has_drift: bool,
    pub drift_summary: Option<String>,
}

