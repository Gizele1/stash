export interface Task {
  id: string;
  name: string;
  status: "active" | "parked";
  current_intent_id: string | null;
  created_at: string;
  parked_at: string | null;
  last_active_at: string;
}

export interface IntentSnapshot {
  id: string;
  task_id: string;
  version: number;
  statement: string;
  trigger: "initial" | "refinement" | "drift_response" | "auto_inferred";
  reason: string | null;
  created_at: string;
}

export interface AgentBranch {
  id: string;
  task_id: string;
  agent_platform: string;
  platform_color: string;
  forked_from_intent_id: string;
  status: "running" | "completed" | "error" | "abandoned";
  progress: number | null;
  output_ref: string | null;
  source_type: "auto" | "manual";
  created_at: string;
  updated_at: string;
}

export interface DriftMarker {
  id: string;
  branch_id: string;
  summary: string;
  created_at: string;
}

export interface ResumeNote {
  id: string;
  task_id: string;
  content: string;
  source: "auto" | "manual";
  created_at: string;
}

export interface EnvironmentSnapshot {
  id: string;
  task_id: string;
  git_branch: string | null;
  git_status: Record<string, unknown> | null;
  git_diff_summary: string | null;
  active_files: string[] | null;
  terminal_last_output: string | null;
  window_focus: Record<string, unknown> | null;
  agent_states: Record<string, unknown> | null;
  captured_at: string;
  completeness: "full" | "partial";
}

export interface AgentEvent {
  id: string;
  branch_id: string;
  event_type: "progress_update" | "completed" | "error" | "commit_detected";
  summary: string | null;
  metadata: Record<string, unknown> | null;
  created_at: string;
  briefing_id: string | null;
}

export interface ReviewLog {
  id: string;
  task_id: string;
  branch_id: string;
  started_at: string;
  duration_seconds: number;
  outcome: "approved" | "rejected" | "rejected_partial";
}

export interface Briefing {
  id: string;
  generated_at: string;
  read_at: string | null;
  items: BriefingItem[];
}

export interface BriefingItem {
  task_id: string;
  task_name: string;
  agent_platform: string;
  event_type: string;
  summary: string;
  has_drift: boolean;
  priority_rank: number;
}

export interface TaskSummary {
  id: string;
  name: string;
  status: "active" | "parked";
  current_intent_statement: string | null;
  agent_count: number;
  running_count: number;
  completed_unreviewed_count: number;
  has_drift: boolean;
  platform_colors: string[];
}

export interface TaskCard {
  task: Task;
  current_intent: IntentSnapshot;
  branches: AgentBranch[];
  resume_note: ResumeNote | null;
  latest_snapshot: EnvironmentSnapshot | null;
  has_drift: boolean;
}

export interface GraphData {
  intent_nodes: {
    id: string;
    version: number;
    statement: string;
    trigger: string;
    created_at: string;
  }[];
  branch_edges: {
    branch_id: string;
    platform: string;
    color: string;
    forked_from_intent_id: string;
    status: string;
    has_drift: boolean;
    drift_summary?: string;
  }[];
  current_intent_id: string;
}
