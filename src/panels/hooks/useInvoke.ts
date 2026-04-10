import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

interface InvokeState<T> {
  data: T | null;
  error: string | null;
  loading: boolean;
}

/**
 * Typed wrapper around Tauri invoke.
 * Returns { data, error, loading, execute } for ergonomic use.
 */
export function useInvoke<T, A extends Record<string, unknown> = Record<string, unknown>>(
  command: string,
) {
  const [state, setState] = useState<InvokeState<T>>({
    data: null,
    error: null,
    loading: false,
  });

  const execute = useCallback(
    async (args?: A): Promise<T | null> => {
      setState({ data: null, error: null, loading: true });
      try {
        const result = await invoke<T>(command, args ?? {});
        setState({ data: result, error: null, loading: false });
        return result;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setState({ data: null, error: message, loading: false });
        return null;
      }
    },
    [command],
  );

  return { ...state, execute };
}
