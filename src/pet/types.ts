// MOD-006: Pet Window Types

/** Sprite sheet configuration for pixel art rendering */
export interface SpriteConfig {
  sheetUrl: string;
  frameWidth: number; // default 64
  frameHeight: number; // default 64
  idleFrames: number[]; // frame indices for idle animation
  fps: number; // default 4
}

/** Status values for a context bubble */
export type BubbleStatus = "running" | "done" | "stuck" | "parked";

/** State for an individual bubble in the ring */
export interface BubbleState {
  contextId: string;
  status: BubbleStatus;
  positionAngle: number; // radians
  color: string;
  isPulsing: boolean;
  isProcessing: boolean;
}

/** Context data returned from Tauri backend */
export interface ContextWithStatus {
  id: string;
  status: BubbleStatus;
  is_processing: boolean;
}

/** Anchor position for card popup */
export interface AnchorPosition {
  x: number;
  y: number;
}

/** Event payload for showing a card */
export interface ShowCardPayload {
  context_id: string;
  anchor_position: AnchorPosition;
}

/** Event payload for state changes */
export interface StateChangePayload {
  contexts: ContextWithStatus[];
}

/** Design system status colors */
export const STATUS_COLORS: Record<BubbleStatus, string> = {
  running: "#534AB7",
  done: "#5cb8a5",
  stuck: "#C4956A",
  parked: "#928f9e",
};

/** Default sprite configuration (placeholder) */
export const DEFAULT_SPRITE_CONFIG: SpriteConfig = {
  sheetUrl: "",
  frameWidth: 64,
  frameHeight: 64,
  idleFrames: [0, 1, 2, 3],
  fps: 4,
};

/**
 * Calculate bubble position angles based on count.
 * Clock positions mapped to radians (0 = 12 o'clock, clockwise).
 *
 * 1 bubble:  6 o'clock
 * 2 bubbles: 4 + 8 o'clock
 * 3 bubbles: 2 + 6 + 10 o'clock
 * 4 bubbles: 1 + 4 + 7 + 10 o'clock
 */
export function getBubbleAngles(count: number): number[] {
  // Convert clock position to radians: clock N -> (N / 12) * 2π
  const clockToRadians = (clock: number): number =>
    (clock / 12) * 2 * Math.PI;

  switch (count) {
    case 0:
      return [];
    case 1:
      return [clockToRadians(6)];
    case 2:
      return [clockToRadians(4), clockToRadians(8)];
    case 3:
      return [clockToRadians(2), clockToRadians(6), clockToRadians(10)];
    case 4:
      return [
        clockToRadians(1),
        clockToRadians(4),
        clockToRadians(7),
        clockToRadians(10),
      ];
    default:
      // For more than 4, distribute evenly
      return Array.from({ length: count }, (_, i) =>
        clockToRadians((i * 12) / count)
      );
  }
}
