import { describe, it, expect } from "vitest";
import {
  STATUS_COLORS,
  DESIGN,
  type ContextWithStatus,
  type Intent,
  type ContextDetail,
  type IntentTimeline,
  type StashConfig,
  type PanelView,
  type ShowCardPayload,
  type ShowGraphPayload,
} from "../types";

describe("Panel types", () => {
  describe("STATUS_COLORS", () => {
    it("maps all four statuses to hex colors", () => {
      const statuses: ContextWithStatus["status"][] = [
        "running",
        "done",
        "stuck",
        "parked",
      ];
      for (const status of statuses) {
        expect(STATUS_COLORS[status]).toMatch(/^#[0-9a-fA-F]{6}$/);
      }
    });

    it("returns correct color for running", () => {
      expect(STATUS_COLORS.running).toBe("#5cb8a5");
    });

    it("returns correct color for done", () => {
      expect(STATUS_COLORS.done).toBe("#534AB7");
    });

    it("returns correct color for stuck", () => {
      expect(STATUS_COLORS.stuck).toBe("#ffb4ab");
    });

    it("returns correct color for parked", () => {
      expect(STATUS_COLORS.parked).toBe("#928f9e");
    });
  });

  describe("DESIGN tokens", () => {
    it("has all required color tokens", () => {
      const requiredColors = [
        "primary",
        "primaryLight",
        "secondary",
        "accent",
        "bg",
        "surface",
        "surfaceHigh",
        "text",
        "textMuted",
        "border",
        "danger",
      ] as const;
      for (const key of requiredColors) {
        expect(DESIGN.colors[key]).toBeDefined();
        expect(DESIGN.colors[key]).toMatch(/^#/);
      }
    });

    it("has all required font tokens", () => {
      expect(DESIGN.fonts.pixel).toContain("Press Start 2P");
      expect(DESIGN.fonts.mono).toContain("IBM Plex Mono");
      expect(DESIGN.fonts.serif).toContain("Noto Serif");
      expect(DESIGN.fonts.sans).toContain("Space Grotesk");
    });

    it("has graph tokens matching spec", () => {
      expect(DESIGN.graph.lineColor).toBe("#534AB7");
      expect(DESIGN.graph.lineWidth).toBe(2);
      expect(DESIGN.graph.nodeSize).toBe(6);
    });
  });

  describe("type shape validation", () => {
    it("ContextWithStatus has required fields", () => {
      const ctx: ContextWithStatus = {
        id: "ctx-1",
        name: "test-project",
        project_dir: "/home/user/project",
        status: "running",
        updated_at: "2026-01-01T00:00:00Z",
      };
      expect(ctx.id).toBe("ctx-1");
      expect(ctx.status).toBe("running");
    });

    it("Intent has required fields", () => {
      const intent: Intent = {
        id: "int-1",
        context_id: "ctx-1",
        tier: "narrative",
        content: "Building the panel window",
        source: "manual",
        created_at: "2026-01-01T00:00:00Z",
        archived: false,
        compressed_from: null,
      };
      expect(intent.tier).toBe("narrative");
      expect(intent.compressed_from).toBeNull();
    });

    it("Intent supports all tier values", () => {
      const tiers: Intent["tier"][] = ["narrative", "summary", "label"];
      for (const tier of tiers) {
        const intent: Intent = {
          id: "int-1",
          context_id: "ctx-1",
          tier,
          content: "test",
          source: "auto",
          created_at: "2026-01-01T00:00:00Z",
          archived: false,
          compressed_from: null,
        };
        expect(intent.tier).toBe(tier);
      }
    });

    it("ContextDetail wraps context and intent", () => {
      const detail: ContextDetail = {
        context: {
          id: "ctx-1",
          name: "test",
          project_dir: "/tmp",
          status: "stuck",
          updated_at: "2026-01-01T00:00:00Z",
        },
        current_intent: null,
        status: "stuck",
      };
      expect(detail.context.status).toBe("stuck");
      expect(detail.current_intent).toBeNull();
    });

    it("IntentTimeline has pagination fields", () => {
      const timeline: IntentTimeline = {
        intents: [],
        has_more: true,
        hidden_count: 5,
      };
      expect(timeline.has_more).toBe(true);
      expect(timeline.hidden_count).toBe(5);
    });

    it("StashConfig has all settings fields", () => {
      const config: StashConfig = {
        llm_mode: "hybrid",
        ollama_url: "http://localhost:11434",
        cloud_api_key: "sk-test",
        pet_position: { x: 100, y: 200 },
        hotkey: "Ctrl+Shift+S",
      };
      expect(config.llm_mode).toBe("hybrid");
      expect(config.pet_position.x).toBe(100);
    });

    it("PanelView covers all view types", () => {
      const views: PanelView[] = ["card", "input", "graph", "settings", "none"];
      expect(views).toHaveLength(5);
    });

    it("ShowCardPayload has context_id and anchor_position", () => {
      const payload: ShowCardPayload = {
        context_id: "ctx-1",
        anchor_position: { x: 50, y: 100 },
      };
      expect(payload.context_id).toBe("ctx-1");
      expect(payload.anchor_position.x).toBe(50);
    });

    it("ShowGraphPayload has context_id", () => {
      const payload: ShowGraphPayload = {
        context_id: "ctx-2",
      };
      expect(payload.context_id).toBe("ctx-2");
    });
  });
});
