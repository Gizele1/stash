import { useEffect } from "react";
import type { ContextWithStatus } from "../types";
import { useInvoke } from "../hooks/useInvoke";

interface ContextSelectorProps {
  value: string;
  onChange: (contextId: string) => void;
}

export function ContextSelector({ value, onChange }: ContextSelectorProps) {
  const { data: contexts, loading, execute } = useInvoke<ContextWithStatus[]>("get_contexts");

  useEffect(() => {
    execute();
  }, [execute]);

  return (
    <select
      className="panels-context-selector"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      disabled={loading}
    >
      <option value="">Select context...</option>
      {contexts?.map((ctx) => (
        <option key={ctx.id} value={ctx.id}>
          {ctx.name} ({ctx.status})
        </option>
      ))}
    </select>
  );
}
