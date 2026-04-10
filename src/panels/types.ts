/** Tauri IPC types for the panel window */

export interface ContextWithStatus {
  id: string;
  name: string;
  project_dir: string;
  status: "running" | "done" | "stuck" | "parked";
  updated_at: string;
}

export interface Intent {
  id: string;
  context_id: string;
  tier: "narrative" | "summary" | "label";
  content: string;
  source: string;
  created_at: string;
  archived: boolean;
  compressed_from: string | null;
}

export interface ContextDetail {
  context: ContextWithStatus;
  current_intent: Intent | null;
  status: string;
}

export interface IntentTimeline {
  intents: Intent[];
  has_more: boolean;
  hidden_count: number;
}

export interface StashConfig {
  llm_mode: "local" | "hybrid" | "cloud";
  ollama_url: string;
  cloud_api_key: string;
  pet_position: { x: number; y: number };
  hotkey: string;
}

/** Panel router view types */
export type PanelView = "card" | "input" | "graph" | "settings" | "none";

export interface ShowCardPayload {
  context_id: string;
  anchor_position: { x: number; y: number };
}

export interface ShowGraphPayload {
  context_id: string;
}

/** Status color mapping */
export const STATUS_COLORS: Record<ContextWithStatus["status"], string> = {
  running: "#5cb8a5",
  done: "#534AB7",
  stuck: "#ffb4ab",
  parked: "#928f9e",
};

/** Design system tokens */
export const DESIGN = {
  colors: {
    primary: "#534AB7",
    primaryLight: "#b1a1ff",
    secondary: "#5cb8a5",
    accent: "#C4956A",
    bg: "#0d0d1a",
    surface: "#1e1e2c",
    surfaceHigh: "#292937",
    text: "#e3e0f4",
    textMuted: "#928f9e",
    border: "#474553",
    danger: "#ffb4ab",
  },
  fonts: {
    pixel: "'Press Start 2P', monospace",
    mono: "'IBM Plex Mono', monospace",
    serif: "'Noto Serif', serif",
    sans: "'Space Grotesk', sans-serif",
  },
  graph: {
    lineColor: "#534AB7",
    lineWidth: 2,
    nodeSize: 6,
  },
} as const;
