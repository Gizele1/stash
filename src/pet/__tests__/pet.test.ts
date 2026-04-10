import { describe, it, expect } from "vitest";
import {
  getBubbleAngles,
  STATUS_COLORS,
  DEFAULT_SPRITE_CONFIG,
} from "../types";
import type { BubbleState, BubbleStatus, ContextWithStatus } from "../types";
import { buildBubbleStates } from "../usePetState";

// Helper: convert radians to approximate clock position
function radiansToClock(radians: number): number {
  return ((radians / (2 * Math.PI)) * 12 + 12) % 12 || 12;
}

describe("getBubbleAngles", () => {
  it("returns empty array for 0 bubbles", () => {
    expect(getBubbleAngles(0)).toEqual([]);
  });

  it("returns 6 o'clock for 1 bubble", () => {
    const angles = getBubbleAngles(1);
    expect(angles).toHaveLength(1);
    expect(radiansToClock(angles[0]!)).toBeCloseTo(6, 5);
  });

  it("returns 4 + 8 o'clock for 2 bubbles", () => {
    const angles = getBubbleAngles(2);
    expect(angles).toHaveLength(2);
    expect(radiansToClock(angles[0]!)).toBeCloseTo(4, 5);
    expect(radiansToClock(angles[1]!)).toBeCloseTo(8, 5);
  });

  it("returns 2 + 6 + 10 o'clock for 3 bubbles", () => {
    const angles = getBubbleAngles(3);
    expect(angles).toHaveLength(3);
    expect(radiansToClock(angles[0]!)).toBeCloseTo(2, 5);
    expect(radiansToClock(angles[1]!)).toBeCloseTo(6, 5);
    expect(radiansToClock(angles[2]!)).toBeCloseTo(10, 5);
  });

  it("returns 1 + 4 + 7 + 10 o'clock for 4 bubbles", () => {
    const angles = getBubbleAngles(4);
    expect(angles).toHaveLength(4);
    expect(radiansToClock(angles[0]!)).toBeCloseTo(1, 5);
    expect(radiansToClock(angles[1]!)).toBeCloseTo(4, 5);
    expect(radiansToClock(angles[2]!)).toBeCloseTo(7, 5);
    expect(radiansToClock(angles[3]!)).toBeCloseTo(10, 5);
  });

  it("distributes evenly for more than 4 bubbles", () => {
    const angles = getBubbleAngles(6);
    expect(angles).toHaveLength(6);
    // 6 bubbles = every 2 hours: 12, 2, 4, 6, 8, 10
    expect(radiansToClock(angles[0]!)).toBeCloseTo(12, 5);
    expect(radiansToClock(angles[1]!)).toBeCloseTo(2, 5);
    expect(radiansToClock(angles[2]!)).toBeCloseTo(4, 5);
    expect(radiansToClock(angles[3]!)).toBeCloseTo(6, 5);
    expect(radiansToClock(angles[4]!)).toBeCloseTo(8, 5);
    expect(radiansToClock(angles[5]!)).toBeCloseTo(10, 5);
  });

  it("all angles are in [0, 2π) range", () => {
    for (let count = 0; count <= 4; count++) {
      const angles = getBubbleAngles(count);
      for (const angle of angles) {
        expect(angle).toBeGreaterThanOrEqual(0);
        expect(angle).toBeLessThan(2 * Math.PI);
      }
    }
  });
});

describe("STATUS_COLORS", () => {
  it("maps running to primary purple", () => {
    expect(STATUS_COLORS.running).toBe("#534AB7");
  });

  it("maps done to secondary teal", () => {
    expect(STATUS_COLORS.done).toBe("#5cb8a5");
  });

  it("maps stuck to accent warm", () => {
    expect(STATUS_COLORS.stuck).toBe("#C4956A");
  });

  it("maps parked to muted", () => {
    expect(STATUS_COLORS.parked).toBe("#928f9e");
  });

  it("covers all BubbleStatus values", () => {
    const statuses: BubbleStatus[] = ["running", "done", "stuck", "parked"];
    for (const status of statuses) {
      expect(STATUS_COLORS[status]).toBeDefined();
      expect(STATUS_COLORS[status]).toMatch(/^#[0-9a-fA-F]{6}$/);
    }
  });
});

describe("DEFAULT_SPRITE_CONFIG", () => {
  it("has 64x64 frame dimensions", () => {
    expect(DEFAULT_SPRITE_CONFIG.frameWidth).toBe(64);
    expect(DEFAULT_SPRITE_CONFIG.frameHeight).toBe(64);
  });

  it("animates at 4fps", () => {
    expect(DEFAULT_SPRITE_CONFIG.fps).toBe(4);
  });

  it("has idle frames defined", () => {
    expect(DEFAULT_SPRITE_CONFIG.idleFrames.length).toBeGreaterThan(0);
  });

  it("has empty sheetUrl (placeholder mode)", () => {
    expect(DEFAULT_SPRITE_CONFIG.sheetUrl).toBe("");
  });
});

describe("BubbleState type validation", () => {
  it("creates a valid BubbleState", () => {
    const state: BubbleState = {
      contextId: "ctx-001",
      status: "running",
      positionAngle: Math.PI,
      color: "#534AB7",
      isPulsing: false,
      isProcessing: false,
    };

    expect(state.contextId).toBe("ctx-001");
    expect(state.status).toBe("running");
    expect(state.positionAngle).toBe(Math.PI);
    expect(state.color).toBe("#534AB7");
    expect(state.isPulsing).toBe(false);
    expect(state.isProcessing).toBe(false);
  });

  it("supports all status values", () => {
    const statuses: BubbleStatus[] = ["running", "done", "stuck", "parked"];
    for (const status of statuses) {
      const state: BubbleState = {
        contextId: `ctx-${status}`,
        status,
        positionAngle: 0,
        color: STATUS_COLORS[status],
        isPulsing: false,
        isProcessing: false,
      };
      expect(state.status).toBe(status);
    }
  });
});

describe("buildBubbleStates", () => {
  it("builds correct bubble states from contexts", () => {
    const contexts: ContextWithStatus[] = [
      { id: "ctx-1", status: "running", is_processing: true },
      { id: "ctx-2", status: "done", is_processing: false },
    ];

    const result = buildBubbleStates(contexts, []);

    expect(result).toHaveLength(2);
    expect(result[0]!.contextId).toBe("ctx-1");
    expect(result[0]!.status).toBe("running");
    expect(result[0]!.color).toBe(STATUS_COLORS.running);
    expect(result[0]!.isProcessing).toBe(true);
    expect(result[0]!.isPulsing).toBe(false); // no previous state

    expect(result[1]!.contextId).toBe("ctx-2");
    expect(result[1]!.status).toBe("done");
    expect(result[1]!.color).toBe(STATUS_COLORS.done);
    expect(result[1]!.isProcessing).toBe(false);
  });

  it("limits to 4 bubbles maximum", () => {
    const contexts: ContextWithStatus[] = Array.from({ length: 6 }, (_, i) => ({
      id: `ctx-${i}`,
      status: "running" as BubbleStatus,
      is_processing: false,
    }));

    const result = buildBubbleStates(contexts, []);
    expect(result).toHaveLength(4);
  });

  it("sets isPulsing when status changes from previous", () => {
    const prevStates: BubbleState[] = [
      {
        contextId: "ctx-1",
        status: "running",
        positionAngle: 0,
        color: STATUS_COLORS.running,
        isPulsing: false,
        isProcessing: false,
      },
    ];

    const contexts: ContextWithStatus[] = [
      { id: "ctx-1", status: "done", is_processing: false },
    ];

    const result = buildBubbleStates(contexts, prevStates);
    expect(result[0]!.isPulsing).toBe(true);
    expect(result[0]!.status).toBe("done");
  });

  it("does not pulse when status unchanged", () => {
    const prevStates: BubbleState[] = [
      {
        contextId: "ctx-1",
        status: "running",
        positionAngle: 0,
        color: STATUS_COLORS.running,
        isPulsing: false,
        isProcessing: false,
      },
    ];

    const contexts: ContextWithStatus[] = [
      { id: "ctx-1", status: "running", is_processing: false },
    ];

    const result = buildBubbleStates(contexts, prevStates);
    expect(result[0]!.isPulsing).toBe(false);
  });

  it("assigns correct angles based on bubble count", () => {
    const contexts: ContextWithStatus[] = [
      { id: "ctx-1", status: "running", is_processing: false },
      { id: "ctx-2", status: "done", is_processing: false },
      { id: "ctx-3", status: "stuck", is_processing: false },
    ];

    const result = buildBubbleStates(contexts, []);
    const expectedAngles = getBubbleAngles(3);

    expect(result[0]!.positionAngle).toBe(expectedAngles[0]);
    expect(result[1]!.positionAngle).toBe(expectedAngles[1]);
    expect(result[2]!.positionAngle).toBe(expectedAngles[2]);
  });

  it("handles empty contexts", () => {
    const result = buildBubbleStates([], []);
    expect(result).toHaveLength(0);
  });
});
