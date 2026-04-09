import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Intent } from "../types";
import { DESIGN } from "../types";

interface IntentNodeProps {
  intent: Intent;
  isDirectionChange?: boolean;
  style?: React.CSSProperties;
}

const TIER_LABELS: Record<Intent["tier"], string> = {
  narrative: "NAR",
  summary: "SUM",
  label: "LBL",
};

function formatTimestamp(iso: string): string {
  const d = new Date(iso);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  return `${hh}:${mm}`;
}

export function IntentNode({ intent, isDirectionChange, style }: IntentNodeProps) {
  const [expanded, setExpanded] = useState(false);
  const [expandedContent, setExpandedContent] = useState<string | null>(null);

  const handleExpand = useCallback(async () => {
    if (!intent.compressed_from) return;
    if (expanded) {
      setExpanded(false);
      return;
    }
    try {
      const result = await invoke<Intent>("expand_compressed_intent", {
        intentId: intent.id,
      });
      setExpandedContent(result.content);
      setExpanded(true);
    } catch {
      // Silently fail on expand error
    }
  }, [intent.id, intent.compressed_from, expanded]);

  const accentStyle = isDirectionChange
    ? { borderLeftColor: DESIGN.colors.accent }
    : {};

  return (
    <div
      className={`panels-intent-node ${isDirectionChange ? "direction-change" : ""} ${intent.compressed_from ? "compressed" : ""}`}
      style={{ ...style, ...accentStyle }}
      data-tier={intent.tier}
    >
      <div className="intent-node-timestamp">
        {formatTimestamp(intent.created_at)}
      </div>
      <div className="intent-node-content">
        <span className="intent-node-tier">{TIER_LABELS[intent.tier]}</span>
        <span className="intent-node-text">
          {expanded && expandedContent ? expandedContent : intent.content}
        </span>
        {intent.compressed_from && (
          <button
            className="intent-node-expand"
            onClick={handleExpand}
            type="button"
          >
            {expanded ? "collapse" : "expand"}
          </button>
        )}
      </div>
    </div>
  );
}

export { formatTimestamp, TIER_LABELS };
