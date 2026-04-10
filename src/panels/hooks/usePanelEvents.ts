import { useState, useEffect, useCallback } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PanelView, ShowCardPayload, ShowGraphPayload } from "../types";

interface PanelEventState {
  view: PanelView;
  cardPayload: ShowCardPayload | null;
  graphPayload: ShowGraphPayload | null;
}

/**
 * Listens to Tauri events and drives the panel router.
 * Returns current view and associated payloads.
 */
export function usePanelEvents() {
  const [state, setState] = useState<PanelEventState>({
    view: "none",
    cardPayload: null,
    graphPayload: null,
  });

  const dismiss = useCallback(() => {
    setState({ view: "none", cardPayload: null, graphPayload: null });
  }, []);

  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    const setup = async () => {
      unlisteners.push(
        await listen<ShowCardPayload>("stash://show-card", (event) => {
          setState({
            view: "card",
            cardPayload: event.payload,
            graphPayload: null,
          });
        }),
      );

      unlisteners.push(
        await listen("stash://show-input", () => {
          setState({
            view: "input",
            cardPayload: null,
            graphPayload: null,
          });
        }),
      );

      unlisteners.push(
        await listen<ShowGraphPayload>("stash://show-graph", (event) => {
          setState({
            view: "graph",
            cardPayload: null,
            graphPayload: event.payload,
          });
        }),
      );

      unlisteners.push(
        await listen("stash://show-settings", () => {
          setState({
            view: "settings",
            cardPayload: null,
            graphPayload: null,
          });
        }),
      );
    };

    setup();

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  return { ...state, dismiss };
}
