import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, emit } from "@tauri-apps/api/event";
import type {
  BubbleState,
  ContextWithStatus,
  StateChangePayload,
  ShowCardPayload,
  AnchorPosition,
} from "./types";
import { STATUS_COLORS, getBubbleAngles } from "./types";

/**
 * Build BubbleState array from raw context data.
 * Assigns position angles based on bubble count using clock positions.
 */
export function buildBubbleStates(
  contexts: ContextWithStatus[],
  prevStates: BubbleState[]
): BubbleState[] {
  const limited = contexts.slice(0, 4);
  const angles = getBubbleAngles(limited.length);

  return limited.map((ctx, i) => {
    const prevState = prevStates.find((b) => b.contextId === ctx.id);
    const statusChanged = prevState ? prevState.status !== ctx.status : false;

    return {
      contextId: ctx.id,
      status: ctx.status,
      positionAngle: angles[i] ?? 0,
      color: STATUS_COLORS[ctx.status],
      isPulsing: statusChanged,
      isProcessing: ctx.is_processing,
    };
  });
}

/**
 * Hook that manages pet window state:
 * - Loads initial contexts from Tauri backend
 * - Listens for state-change events
 * - Provides bubble click handler (emits show-card event)
 * - Provides pet drag handler (saves position)
 */
export function usePetState() {
  const [bubbles, setBubbles] = useState<BubbleState[]>([]);

  // Load initial contexts
  useEffect(() => {
    let cancelled = false;

    invoke<ContextWithStatus[]>("get_contexts")
      .then((contexts) => {
        if (!cancelled) {
          setBubbles((prev) => buildBubbleStates(contexts, prev));
        }
      })
      .catch(() => {
        // Command may not be registered yet — that's expected
      });

    return () => {
      cancelled = true;
    };
  }, []);

  // Listen for state changes
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<StateChangePayload>("stash://state-change", (event) => {
      setBubbles((prev) => buildBubbleStates(event.payload.contexts, prev));
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // Event system may not be available in tests
      });

    return () => {
      unlisten?.();
    };
  }, []);

  // Clear pulse after animation completes
  useEffect(() => {
    const pulsingBubbles = bubbles.filter((b) => b.isPulsing);
    if (pulsingBubbles.length === 0) return;

    const timer = setTimeout(() => {
      setBubbles((prev) => prev.map((b) => ({ ...b, isPulsing: false })));
    }, 1500); // match CSS animation duration

    return () => clearTimeout(timer);
  }, [bubbles]);

  // Handle bubble click — emit show-card event
  const handleBubbleClick = useCallback(
    (contextId: string, anchorPosition: AnchorPosition) => {
      const payload: ShowCardPayload = {
        context_id: contextId,
        anchor_position: anchorPosition,
      };
      emit("stash://show-card", payload).catch(() => {
        // Event system may not be available
      });
    },
    []
  );

  // Handle pet drag end — save position
  const handleDragEnd = useCallback((x: number, y: number) => {
    invoke("save_pet_position", { x, y }).catch(() => {
      // Command may not be registered yet
    });
  }, []);

  return {
    bubbles,
    handleBubbleClick,
    handleDragEnd,
  };
}
