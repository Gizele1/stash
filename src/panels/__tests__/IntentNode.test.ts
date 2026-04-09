import { describe, it, expect } from "vitest";
import type { Intent } from "../types";

// Import the pure functions exported from IntentNode
// We test the helper logic without rendering the component
import { formatTimestamp, TIER_LABELS } from "../components/IntentNode";
import { detectDirectionChanges } from "../IntentGraphView";

function makeIntent(overrides: Partial<Intent> = {}): Intent {
  return {
    id: "int-1",
    context_id: "ctx-1",
    tier: "label",
    content: "Default intent content",
    source: "auto",
    created_at: "2026-04-09T14:30:00Z",
    archived: false,
    compressed_from: null,
    ...overrides,
  };
}

describe("IntentNode helpers", () => {
  describe("formatTimestamp", () => {
    it("formats ISO string to HH:MM", () => {
      // This depends on local timezone, so we just check format
      const result = formatTimestamp("2026-04-09T14:30:00Z");
      expect(result).toMatch(/^\d{2}:\d{2}$/);
    });

    it("pads single-digit hours/minutes", () => {
      const result = formatTimestamp("2026-01-01T03:05:00Z");
      expect(result).toMatch(/^\d{2}:\d{2}$/);
    });
  });

  describe("TIER_LABELS", () => {
    it("maps narrative to NAR", () => {
      expect(TIER_LABELS.narrative).toBe("NAR");
    });

    it("maps summary to SUM", () => {
      expect(TIER_LABELS.summary).toBe("SUM");
    });

    it("maps label to LBL", () => {
      expect(TIER_LABELS.label).toBe("LBL");
    });
  });
});

describe("detectDirectionChanges", () => {
  it("returns empty set for empty list", () => {
    const result = detectDirectionChanges([]);
    expect(result.size).toBe(0);
  });

  it("returns empty set for single intent", () => {
    const result = detectDirectionChanges([makeIntent()]);
    expect(result.size).toBe(0);
  });

  it("detects tier upgrade from label to narrative", () => {
    const intents = [
      makeIntent({ id: "1", tier: "label", content: "Setup" }),
      makeIntent({ id: "2", tier: "narrative", content: "Building new feature" }),
    ];
    const result = detectDirectionChanges(intents);
    expect(result.has("2")).toBe(true);
  });

  it("detects tier upgrade from summary to narrative", () => {
    const intents = [
      makeIntent({ id: "1", tier: "summary", content: "Overview" }),
      makeIntent({ id: "2", tier: "narrative", content: "Deep dive" }),
    ];
    const result = detectDirectionChanges(intents);
    expect(result.has("2")).toBe(true);
  });

  it("does not flag same-tier transitions", () => {
    const intents = [
      makeIntent({ id: "1", tier: "label", content: "Step A" }),
      makeIntent({ id: "2", tier: "label", content: "Step B" }),
    ];
    const result = detectDirectionChanges(intents);
    expect(result.has("2")).toBe(false);
  });

  it("detects content-based direction change for narrative", () => {
    const intents = [
      makeIntent({
        id: "1",
        tier: "narrative",
        content: "Working on frontend components",
      }),
      makeIntent({
        id: "2",
        tier: "narrative",
        content: "Switching to backend API",
      }),
    ];
    const result = detectDirectionChanges(intents);
    expect(result.has("2")).toBe(true);
  });

  it("does not flag content-based change for non-narrative", () => {
    const intents = [
      makeIntent({
        id: "1",
        tier: "label",
        content: "Working on frontend components",
      }),
      makeIntent({
        id: "2",
        tier: "label",
        content: "Switching to backend API completely different",
      }),
    ];
    const result = detectDirectionChanges(intents);
    expect(result.has("2")).toBe(false);
  });

  it("handles multiple direction changes in a sequence", () => {
    const intents = [
      makeIntent({ id: "1", tier: "label", content: "Init" }),
      makeIntent({ id: "2", tier: "narrative", content: "First direction" }),
      makeIntent({ id: "3", tier: "label", content: "Checkpoint" }),
      makeIntent({ id: "4", tier: "narrative", content: "Second direction" }),
    ];
    const result = detectDirectionChanges(intents);
    expect(result.has("2")).toBe(true);
    expect(result.has("4")).toBe(true);
    expect(result.size).toBe(2);
  });
});
