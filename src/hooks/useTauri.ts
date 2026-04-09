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
};
