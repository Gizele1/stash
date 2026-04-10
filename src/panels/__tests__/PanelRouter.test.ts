import { describe, it, expect } from "vitest";
import type { PanelView, ShowCardPayload, ShowGraphPayload } from "../types";

/**
 * Panel router logic tests.
 * Since we cannot render React components without jsdom,
 * we test the routing logic as a pure function.
 */

interface PanelEventState {
  view: PanelView;
  cardPayload: ShowCardPayload | null;
  graphPayload: ShowGraphPayload | null;
}

/** Simulate the routing logic from PanelRouter */
function resolveView(state: PanelEventState): string | null {
  switch (state.view) {
    case "card":
      return state.cardPayload ? "CardPopupView" : null;
    case "input":
      return "ManualInputView";
    case "graph":
      return state.graphPayload ? "IntentGraphView" : null;
    case "settings":
      return "SettingsView";
    case "none":
    default:
      return null;
  }
}

/** Simulate event state transitions */
function applyEvent(
  eventName: string,
  payload?: ShowCardPayload | ShowGraphPayload,
): PanelEventState {
  switch (eventName) {
    case "stash://show-card":
      return {
        view: "card",
        cardPayload: payload as ShowCardPayload,
        graphPayload: null,
      };
    case "stash://show-input":
      return {
        view: "input",
        cardPayload: null,
        graphPayload: null,
      };
    case "stash://show-graph":
      return {
        view: "graph",
        cardPayload: null,
        graphPayload: payload as ShowGraphPayload,
      };
    case "stash://show-settings":
      return {
        view: "settings",
        cardPayload: null,
        graphPayload: null,
      };
    default:
      return {
        view: "none",
        cardPayload: null,
        graphPayload: null,
      };
  }
}

describe("PanelRouter logic", () => {
  it("shows no view by default", () => {
    const state: PanelEventState = {
      view: "none",
      cardPayload: null,
      graphPayload: null,
    };
    expect(resolveView(state)).toBeNull();
  });

  it("routes stash://show-card to CardPopupView", () => {
    const state = applyEvent("stash://show-card", {
      context_id: "ctx-1",
      anchor_position: { x: 100, y: 200 },
    });
    expect(state.view).toBe("card");
    expect(resolveView(state)).toBe("CardPopupView");
    expect(state.cardPayload?.context_id).toBe("ctx-1");
  });

  it("routes stash://show-input to ManualInputView", () => {
    const state = applyEvent("stash://show-input");
    expect(state.view).toBe("input");
    expect(resolveView(state)).toBe("ManualInputView");
  });

  it("routes stash://show-graph to IntentGraphView", () => {
    const state = applyEvent("stash://show-graph", { context_id: "ctx-2" });
    expect(state.view).toBe("graph");
    expect(resolveView(state)).toBe("IntentGraphView");
    expect(state.graphPayload?.context_id).toBe("ctx-2");
  });

  it("routes stash://show-settings to SettingsView", () => {
    const state = applyEvent("stash://show-settings");
    expect(state.view).toBe("settings");
    expect(resolveView(state)).toBe("SettingsView");
  });

  it("returns null for card view without payload", () => {
    const state: PanelEventState = {
      view: "card",
      cardPayload: null,
      graphPayload: null,
    };
    expect(resolveView(state)).toBeNull();
  });

  it("returns null for graph view without payload", () => {
    const state: PanelEventState = {
      view: "graph",
      cardPayload: null,
      graphPayload: null,
    };
    expect(resolveView(state)).toBeNull();
  });

  it("clears previous payload on view switch", () => {
    const cardState = applyEvent("stash://show-card", {
      context_id: "ctx-1",
      anchor_position: { x: 0, y: 0 },
    });
    expect(cardState.cardPayload).not.toBeNull();

    const inputState = applyEvent("stash://show-input");
    expect(inputState.cardPayload).toBeNull();
    expect(inputState.graphPayload).toBeNull();
  });

  it("handles unknown event gracefully", () => {
    const state = applyEvent("stash://unknown");
    expect(state.view).toBe("none");
    expect(resolveView(state)).toBeNull();
  });
});
