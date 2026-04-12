import { invoke } from "@tauri-apps/api/core";
import type {
  Task,
  TaskCard,
  IntentSnapshot,
  AgentBranch,
  DriftMarker,
  ResumeNote,
  Briefing,
  GraphData,
  ReviewLog,
  ContextWithStatus,
  ContextDetail,
  IntentTimeline,
  FocusResult,
  PrUrlResult,
} from "../types/models";

// Re-export the Rust backend's TaskCardData as TaskCard
export const api = {
  taskCreate: (name: string, initialIntent: string) =>
    invoke<TaskCard>("task_create", { name, initialIntent }),

  taskList: (status?: string) =>
    invoke<Task[]>("task_list", { status: status ?? null }),

  taskGetCard: (taskId: string) =>
    invoke<TaskCard>("task_get_card", { taskId }),

  taskSwitch: (taskId: string) =>
    invoke<Task>("task_switch", { taskId }),

  taskPark: (taskId: string) =>
    invoke<Task>("task_park", { taskId }),

  refineIntent: (
    taskId: string,
    statement: string,
    triggerType: string,
    reason?: string
  ) =>
    invoke<IntentSnapshot>("refine_intent", {
      taskId,
      statement,
      triggerType,
      reason: reason ?? null,
    }),

  createManualBranch: (
    taskId: string,
    agentPlatform: string,
    platformColor: string
  ) =>
    invoke<AgentBranch>("create_manual_branch", {
      taskId,
      agentPlatform,
      platformColor,
    }),

  updateAgentBranch: (
    branchId: string,
    status?: string,
    progress?: number,
    outputRef?: string
  ) =>
    invoke<AgentBranch>("update_agent_branch", {
      branchId,
      status: status ?? null,
      progress: progress ?? null,
      outputRef: outputRef ?? null,
    }),

  markDrift: (branchId: string, summary: string) =>
    invoke<DriftMarker>("mark_drift", { branchId, summary }),

  saveResumeNote: (taskId: string, content: string, source: string) =>
    invoke<ResumeNote>("save_resume_note", { taskId, content, source }),

  getBriefing: () => invoke<Briefing>("get_briefing"),

  getGraphData: (taskId: string) =>
    invoke<GraphData>("get_graph_data", { taskId }),

  parkAllTasks: () => invoke<void>("park_all_tasks"),

  startReview: (taskId: string, branchId: string) =>
    invoke<string>("start_review", { taskId, branchId }),

  endReview: (
    taskId: string,
    branchId: string,
    startedAt: string,
    durationSeconds: number,
    outcome: string
  ) =>
    invoke<ReviewLog>("end_review", {
      taskId,
      branchId,
      startedAt,
      durationSeconds,
      outcome,
    }),

  setTaskDependency: (fromTaskId: string, toTaskId: string) =>
    invoke<void>("set_task_dependency", { fromTaskId, toTaskId }),

  queryReviewLogs: (taskId?: string, date?: string) =>
    invoke<ReviewLog[]>("query_review_logs", {
      taskId: taskId ?? null,
      date: date ?? null,
    }),

  getUnreviewedBranchCount: () =>
    invoke<number>("get_unreviewed_branch_count"),

  openExternal: (url: string) =>
    invoke<void>("open_external", { url }),

  openGraphWindow: (taskId: string) =>
    invoke<void>("open_graph_window", { taskId }),

  // ── v2 API (Brain-based context engine) ──

  v2GetContexts: () =>
    invoke<ContextWithStatus[]>("get_contexts"),

  v2GetContextDetail: (contextId: string) =>
    invoke<ContextDetail>("get_context_detail", { contextId }),

  v2GetIntentTimeline: (contextId: string, limit?: number, beforeId?: string) =>
    invoke<IntentTimeline>("get_intent_timeline", {
      contextId,
      limit: limit ?? null,
      beforeId: beforeId ?? null,
    }),

  v2OverrideStatus: (contextId: string, newStatus: string) =>
    invoke<boolean>("override_status", { contextId, newStatus }),

  v2SubmitManualIntent: (contextId: string, content: string) =>
    invoke<string>("submit_manual_intent", { contextId, content }),

  v2CorrectIntent: (intentId: string, newContent: string) =>
    invoke<string>("correct_intent", { intentId, newContent }),

  v2FocusTerminal: (projectDir: string) =>
    invoke<FocusResult>("focus_terminal", { projectDir }),

  v2OpenPrUrl: (projectDir: string) =>
    invoke<PrUrlResult>("open_pr_url", { projectDir }),
};
