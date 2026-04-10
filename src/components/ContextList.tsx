import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ContextWithStatus, ContextDetail } from "../types/models";
import { api } from "../hooks/useTauri";

const STATUS_COLORS: Record<string, string> = {
  running: "#22c55e",
  done: "#6b7280",
  stuck: "#ef4444",
  parked: "#eab308",
};

const STATUS_LABELS: Record<string, string> = {
  running: "Running",
  done: "Done",
  stuck: "Stuck",
  parked: "Parked",
};

interface Props {
  onSelectContext?: (contextId: string) => void;
}

export function ContextList({ onSelectContext }: Props) {
  const [contexts, setContexts] = useState<ContextWithStatus[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<ContextDetail | null>(null);
  const [intentInput, setIntentInput] = useState("");
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const result = await api.v2GetContexts();
      setContexts(result);
    } catch (e) {
      console.error("Failed to load contexts:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    const unlisten = listen("stash://jsonl-messages", () => refresh());
    const unlistenGit = listen("stash://git-signal", () => refresh());
    return () => {
      clearInterval(interval);
      unlisten.then((fn) => fn());
      unlistenGit.then((fn) => fn());
    };
  }, [refresh]);

  const handleSelect = async (ctx: ContextWithStatus) => {
    setSelectedId(ctx.id);
    onSelectContext?.(ctx.id);
    try {
      const d = await api.v2GetContextDetail(ctx.id);
      setDetail(d);
    } catch (e) {
      console.error("Failed to load context detail:", e);
    }
  };

  const handleOverrideStatus = async (contextId: string, status: string) => {
    try {
      await api.v2OverrideStatus(contextId, status);
      refresh();
      if (selectedId === contextId) {
        const d = await api.v2GetContextDetail(contextId);
        setDetail(d);
      }
    } catch (e) {
      console.error("Failed to override status:", e);
    }
  };

  const handleSubmitIntent = async () => {
    if (!selectedId || !intentInput.trim()) return;
    try {
      await api.v2SubmitManualIntent(selectedId, intentInput.trim());
      setIntentInput("");
      const d = await api.v2GetContextDetail(selectedId);
      setDetail(d);
    } catch (e) {
      console.error("Failed to submit intent:", e);
    }
  };

  const handleFocusTerminal = async (projectDir: string) => {
    try {
      await api.v2FocusTerminal(projectDir);
    } catch (e) {
      console.error("Failed to focus terminal:", e);
    }
  };

  const activeContexts = contexts.filter((c) => c.status === "running" || c.status === "stuck");
  const inactiveContexts = contexts.filter((c) => c.status === "done" || c.status === "parked");

  return (
    <div style={{ padding: 16, maxWidth: 420, fontFamily: "system-ui, -apple-system, sans-serif" }}>
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 12 }}>
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 700 }}>Stash v2</h2>
        <span style={{ fontSize: 11, color: "#9ca3af" }}>
          {contexts.length} context{contexts.length !== 1 ? "s" : ""}
        </span>
      </div>

      {loading ? (
        <p style={{ color: "#9ca3af", fontSize: 13 }}>Loading...</p>
      ) : contexts.length === 0 ? (
        <p style={{ color: "#9ca3af", fontSize: 13 }}>
          No contexts yet. Start a Claude Code session to auto-detect.
        </p>
      ) : (
        <>
          {/* Active contexts */}
          {activeContexts.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <h3 style={{ fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "0 0 6px", letterSpacing: 0.5 }}>
                Active ({activeContexts.length})
              </h3>
              {activeContexts.map((ctx) => (
                <ContextCard
                  key={ctx.id}
                  ctx={ctx}
                  selected={selectedId === ctx.id}
                  onSelect={() => handleSelect(ctx)}
                  onOverride={handleOverrideStatus}
                  onFocusTerminal={handleFocusTerminal}
                />
              ))}
            </div>
          )}

          {/* Inactive contexts */}
          {inactiveContexts.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <h3 style={{ fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "0 0 6px", letterSpacing: 0.5 }}>
                Inactive ({inactiveContexts.length})
              </h3>
              {inactiveContexts.map((ctx) => (
                <ContextCard
                  key={ctx.id}
                  ctx={ctx}
                  selected={selectedId === ctx.id}
                  onSelect={() => handleSelect(ctx)}
                  onOverride={handleOverrideStatus}
                  onFocusTerminal={handleFocusTerminal}
                />
              ))}
            </div>
          )}
        </>
      )}

      {/* Detail panel */}
      {detail && selectedId && (
        <div style={{ marginTop: 12, padding: 10, background: "#f9fafb", borderRadius: 6, border: "1px solid #e5e7eb" }}>
          <h4 style={{ margin: "0 0 6px", fontSize: 13, fontWeight: 600 }}>
            {detail.context.name}
          </h4>
          {detail.current_intent ? (
            <div style={{ fontSize: 12, color: "#374151", marginBottom: 8 }}>
              <span style={{ color: "#6b7280", fontSize: 11 }}>Current intent:</span>
              <br />
              {detail.current_intent.content}
            </div>
          ) : (
            <p style={{ fontSize: 12, color: "#9ca3af", marginBottom: 8 }}>No intent yet.</p>
          )}

          {/* Manual intent input */}
          <div style={{ display: "flex", gap: 6 }}>
            <input
              type="text"
              value={intentInput}
              onChange={(e) => setIntentInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSubmitIntent()}
              placeholder="Add manual intent..."
              style={{
                flex: 1,
                fontSize: 12,
                padding: "4px 8px",
                border: "1px solid #d1d5db",
                borderRadius: 4,
                outline: "none",
              }}
            />
            <button
              onClick={handleSubmitIntent}
              disabled={!intentInput.trim()}
              style={{
                fontSize: 12,
                padding: "4px 10px",
                background: intentInput.trim() ? "#3b82f6" : "#d1d5db",
                color: "#fff",
                border: "none",
                borderRadius: 4,
                cursor: intentInput.trim() ? "pointer" : "default",
              }}
            >
              Add
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── ContextCard sub-component ──

interface ContextCardProps {
  ctx: ContextWithStatus;
  selected: boolean;
  onSelect: () => void;
  onOverride: (contextId: string, status: string) => void;
  onFocusTerminal: (projectDir: string) => void;
}

function ContextCard({ ctx, selected, onSelect, onOverride, onFocusTerminal }: ContextCardProps) {
  const dirName = ctx.project_dir.split("/").pop() || ctx.project_dir;
  const timeAgo = formatRelativeTime(ctx.updated_at);

  return (
    <div
      onClick={onSelect}
      style={{
        padding: "8px 10px",
        marginBottom: 4,
        borderRadius: 6,
        border: selected ? "1px solid #3b82f6" : "1px solid #e5e7eb",
        background: selected ? "#eff6ff" : "#fff",
        cursor: "pointer",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: STATUS_COLORS[ctx.status] || "#9ca3af",
              display: "inline-block",
              flexShrink: 0,
            }}
          />
          <span style={{ fontSize: 13, fontWeight: 500 }}>{ctx.name || dirName}</span>
        </div>
        <span style={{ fontSize: 10, color: "#9ca3af" }}>{timeAgo}</span>
      </div>

      <div style={{ fontSize: 11, color: "#6b7280", marginTop: 2, marginLeft: 14 }}>
        {dirName}
        <span style={{ marginLeft: 6, color: STATUS_COLORS[ctx.status] }}>
          {STATUS_LABELS[ctx.status] || ctx.status}
        </span>
      </div>

      {/* Action buttons (shown when selected) */}
      {selected && (
        <div style={{ marginTop: 6, marginLeft: 14, display: "flex", gap: 4 }}>
          {ctx.status === "running" && (
            <ActionBtn label="Park" onClick={() => onOverride(ctx.id, "parked")} />
          )}
          {ctx.status === "parked" && (
            <ActionBtn label="Resume" onClick={() => onOverride(ctx.id, "running")} />
          )}
          {ctx.status === "stuck" && (
            <ActionBtn label="Unstuck" onClick={() => onOverride(ctx.id, "running")} />
          )}
          <ActionBtn label="Focus" onClick={() => onFocusTerminal(ctx.project_dir)} />
        </div>
      )}
    </div>
  );
}

function ActionBtn({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      onClick={(e) => { e.stopPropagation(); onClick(); }}
      style={{
        fontSize: 10,
        padding: "2px 8px",
        background: "none",
        border: "1px solid #d1d5db",
        borderRadius: 3,
        cursor: "pointer",
        color: "#374151",
      }}
    >
      {label}
    </button>
  );
}

function formatRelativeTime(isoString: string): string {
  try {
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return "just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    const diffDays = Math.floor(diffHours / 24);
    return `${diffDays}d ago`;
  } catch {
    return "";
  }
}
