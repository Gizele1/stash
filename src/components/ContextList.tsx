import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ContextWithStatus, ContextDetail, IntentTimeline, IntentRecord } from "../types/models";
import { api } from "../hooks/useTauri";

const STATUS_COLORS: Record<string, string> = {
  running: "var(--status-running)",
  done: "var(--status-done)",
  stuck: "var(--status-stuck)",
  parked: "var(--status-parked)",
};

const STATUS_LABELS: Record<string, string> = {
  running: "RUNNING",
  done: "DONE",
  stuck: "STUCK",
  parked: "PARKED",
};

export function ContextList() {
  const [contexts, setContexts] = useState<ContextWithStatus[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<ContextDetail | null>(null);
  const [timeline, setTimeline] = useState<IntentTimeline | null>(null);
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
    const unlisten = listen("stash://state-change", () => refresh());
    return () => {
      clearInterval(interval);
      unlisten.then((fn) => fn());
    };
  }, [refresh]);

  const handleSelect = async (ctx: ContextWithStatus) => {
    setSelectedId(ctx.id);
    try {
      const [d, t] = await Promise.all([
        api.v2GetContextDetail(ctx.id),
        api.v2GetIntentTimeline(ctx.id, 20),
      ]);
      setDetail(d);
      setTimeline(t);
    } catch (e) {
      console.error("Failed to load detail:", e);
    }
  };

  const handleOverride = async (contextId: string, status: string) => {
    try {
      await api.v2OverrideStatus(contextId, status);
      refresh();
      if (selectedId === contextId) {
        const d = await api.v2GetContextDetail(contextId);
        setDetail(d);
      }
    } catch (e) {
      console.error("Override failed:", e);
    }
  };

  const handleSubmitIntent = async () => {
    if (!selectedId || !intentInput.trim()) return;
    try {
      await api.v2SubmitManualIntent(selectedId, intentInput.trim());
      setIntentInput("");
      const [d, t] = await Promise.all([
        api.v2GetContextDetail(selectedId),
        api.v2GetIntentTimeline(selectedId, 20),
      ]);
      setDetail(d);
      setTimeline(t);
    } catch (e) {
      console.error("Submit failed:", e);
    }
  };

  return (
    <div style={{ display: "flex", height: "calc(100vh - 40px)" }}>
      {/* Left: Context List */}
      <div
        style={{
          width: 320,
          borderRight: "2px solid var(--color-border-muted)",
          overflow: "auto",
          padding: 16,
          flexShrink: 0,
        }}
      >
        <SectionLabel text="ACTIVE CONTEXTS" count={contexts.length} />

        {loading ? (
          <EmptyState text="LOADING..." />
        ) : contexts.length === 0 ? (
          <EmptyState text="NO CONTEXTS DETECTED. START A CLAUDE CODE SESSION." />
        ) : (
          contexts.map((ctx) => (
            <ContextCard
              key={ctx.id}
              ctx={ctx}
              selected={selectedId === ctx.id}
              onSelect={() => handleSelect(ctx)}
              onOverride={handleOverride}
            />
          ))
        )}
      </div>

      {/* Right: Detail + Timeline */}
      <div style={{ flex: 1, overflow: "auto", padding: 24 }}>
        {detail && selectedId ? (
          <DetailPanel
            detail={detail}
            timeline={timeline}
            intentInput={intentInput}
            onInputChange={setIntentInput}
            onSubmit={handleSubmitIntent}
          />
        ) : (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              height: "100%",
              color: "var(--color-text-muted)",
              fontFamily: "var(--font-pixel)",
              fontSize: 9,
            }}
          >
            SELECT A CONTEXT
          </div>
        )}
      </div>
    </div>
  );
}

/* ── Sub-components ── */

function SectionLabel({ text, count }: { text: string; count: number }) {
  return (
    <div
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
        marginBottom: 12,
      }}
    >
      <span
        style={{
          fontFamily: "var(--font-pixel)",
          fontSize: 8,
          color: "var(--color-primary-light)",
          letterSpacing: "0.1em",
        }}
      >
        {text}
      </span>
      <span
        style={{
          fontFamily: "var(--font-pixel)",
          fontSize: 7,
          color: "var(--color-text-muted)",
        }}
      >
        {count}/4
      </span>
    </div>
  );
}

function EmptyState({ text }: { text: string }) {
  return (
    <p
      style={{
        fontFamily: "var(--font-pixel)",
        fontSize: 7,
        color: "var(--color-text-muted)",
        lineHeight: 1.8,
        marginTop: 32,
        textAlign: "center",
      }}
    >
      {text}
    </p>
  );
}

interface ContextCardProps {
  ctx: ContextWithStatus;
  selected: boolean;
  onSelect: () => void;
  onOverride: (id: string, status: string) => void;
}

function ContextCard({ ctx, selected, onSelect, onOverride }: ContextCardProps) {
  const dirName = ctx.project_dir.split("/").pop() || ctx.name;
  const timeAgo = formatRelativeTime(ctx.updated_at);
  const statusColor = STATUS_COLORS[ctx.status] || "var(--color-text-muted)";

  return (
    <div
      onClick={onSelect}
      style={{
        border: `2px solid ${selected ? "var(--color-primary-light)" : "var(--color-border-muted)"}`,
        background: selected ? "var(--color-surface-container)" : "var(--color-surface)",
        padding: 12,
        marginBottom: 6,
        cursor: "pointer",
        position: "relative",
        transition: "border-color 0.15s",
      }}
    >
      {/* Status tag */}
      <div
        style={{
          position: "absolute",
          top: -1,
          right: -1,
          background: statusColor,
          padding: "2px 6px",
          border: "2px solid #000",
        }}
      >
        <span
          style={{
            fontFamily: "var(--font-pixel)",
            fontSize: 6,
            color: ctx.status === "stuck" ? "#000" : "#fff",
            fontWeight: 700,
          }}
        >
          {STATUS_LABELS[ctx.status]}
        </span>
      </div>

      {/* Project name */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
        <div
          style={{
            width: 8,
            height: 8,
            background: statusColor,
            flexShrink: 0,
          }}
        />
        <span
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            fontWeight: 600,
            color: "var(--color-text)",
          }}
        >
          {ctx.name || dirName}
        </span>
      </div>

      {/* Path + time */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          color: "var(--color-text-muted)",
          marginLeft: 16,
        }}
      >
        {dirName}
        <span style={{ marginLeft: 8, opacity: 0.6 }}>{timeAgo}</span>
      </div>

      {/* Action buttons */}
      {selected && (
        <div style={{ marginTop: 8, marginLeft: 16, display: "flex", gap: 6 }}>
          {ctx.status === "running" && (
            <SmallBtn label="PARK" onClick={() => onOverride(ctx.id, "parked")} />
          )}
          {ctx.status === "parked" && (
            <SmallBtn label="RESUME" onClick={() => onOverride(ctx.id, "running")} />
          )}
          {ctx.status === "stuck" && (
            <SmallBtn label="UNSTUCK" onClick={() => onOverride(ctx.id, "running")} />
          )}
        </div>
      )}
    </div>
  );
}

function SmallBtn({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
      style={{
        fontFamily: "var(--font-pixel)",
        fontSize: 7,
        padding: "3px 8px",
        background: "none",
        border: "1px solid var(--color-border)",
        color: "var(--color-primary-light)",
        cursor: "pointer",
        letterSpacing: "0.05em",
      }}
    >
      {label}
    </button>
  );
}

/* ── Detail Panel ── */

interface DetailPanelProps {
  detail: ContextDetail;
  timeline: IntentTimeline | null;
  intentInput: string;
  onInputChange: (v: string) => void;
  onSubmit: () => void;
}

function DetailPanel({ detail, timeline, intentInput, onInputChange, onSubmit }: DetailPanelProps) {
  return (
    <div>
      {/* Context header */}
      <div style={{ marginBottom: 20 }}>
        <h2
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 20,
            fontWeight: 700,
            margin: "0 0 4px",
            color: "var(--color-text)",
          }}
        >
          {detail.context.name}
        </h2>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-muted)",
          }}
        >
          {detail.context.project_dir}
        </span>
      </div>

      {/* Current intent card */}
      <div className="section-box" style={{ position: "relative", paddingTop: 24 }}>
        <div
          style={{
            position: "absolute",
            top: -1,
            left: -1,
            background: detail.current_intent
              ? "var(--color-accent)"
              : "var(--color-border)",
            border: "2px solid #000",
            padding: "2px 6px",
          }}
        >
          <span
            style={{
              fontFamily: "var(--font-pixel)",
              fontSize: 7,
              color: "#000",
              fontWeight: 700,
            }}
          >
            {detail.current_intent?.source === "manual" ? "MANUAL" : "AUTO"}
          </span>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
          <div style={{ width: 10, height: 10, background: "var(--color-primary)", opacity: 0.8 }} />
          <span
            style={{
              fontFamily: "var(--font-pixel)",
              fontSize: 8,
              color: "var(--color-primary-light)",
            }}
          >
            MISSION_LOG
          </span>
        </div>

        <p
          style={{
            fontSize: 13,
            lineHeight: 1.6,
            fontWeight: 700,
            color: "#e8e8f0",
            margin: 0,
          }}
        >
          {detail.current_intent?.content || "No intent distilled yet. Keep coding..."}
        </p>
      </div>

      {/* Manual intent input */}
      <div style={{ display: "flex", gap: 8, marginBottom: 24 }}>
        <input
          type="text"
          value={intentInput}
          onChange={(e) => onInputChange(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && onSubmit()}
          placeholder="Manual intent..."
          style={{
            flex: 1,
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            padding: "8px 12px",
            background: "var(--color-surface)",
            border: "2px solid var(--color-border-muted)",
            color: "var(--color-text)",
            outline: "none",
          }}
        />
        <button
          className="arcade-btn"
          onClick={onSubmit}
          disabled={!intentInput.trim()}
          style={{ opacity: intentInput.trim() ? 1 : 0.4 }}
        >
          ADD
        </button>
      </div>

      {/* Intent Timeline */}
      {timeline && timeline.intents.length > 0 && (
        <IntentTimelineView
          intents={timeline.intents}
          hiddenCount={timeline.hidden_count}
        />
      )}
    </div>
  );
}

/* ── Intent Timeline (editorial style with spine) ── */

function IntentTimelineView({ intents, hiddenCount }: { intents: IntentRecord[]; hiddenCount: number }) {
  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 16 }}>
        <span
          style={{
            fontFamily: "var(--font-pixel)",
            fontSize: 8,
            color: "var(--color-primary-light)",
            letterSpacing: "0.1em",
          }}
        >
          INTENT TIMELINE
        </span>
        <div style={{ flex: 1, height: 1, background: "var(--color-border-muted)", opacity: 0.3 }} />
        {hiddenCount > 0 && (
          <span style={{ fontFamily: "var(--font-pixel)", fontSize: 7, color: "var(--color-text-muted)" }}>
            +{hiddenCount} ARCHIVED
          </span>
        )}
      </div>

      <div style={{ position: "relative", paddingLeft: 48 }}>
        {/* Spine */}
        <div
          style={{
            position: "absolute",
            left: 24,
            top: 0,
            bottom: 0,
            width: 2,
            background: "var(--color-primary)",
          }}
        />

        {intents.map((intent, i) => (
          <IntentNode key={intent.id} intent={intent} isFirst={i === 0} />
        ))}
      </div>
    </div>
  );
}

function IntentNode({ intent, isFirst }: { intent: IntentRecord; isFirst: boolean }) {
  const time = formatTime(intent.created_at);
  const tierColors: Record<string, string> = {
    narrative: "var(--color-primary)",
    summary: "var(--color-secondary)",
    label: "var(--color-accent)",
  };
  const nodeColor = tierColors[intent.tier] || "var(--color-primary)";

  return (
    <div style={{ position: "relative", marginBottom: 16, display: "flex", alignItems: "flex-start" }}>
      {/* Time label */}
      <div
        style={{
          position: "absolute",
          left: -48,
          width: 40,
          textAlign: "right",
          paddingTop: 4,
        }}
      >
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 9, color: "var(--color-text-muted)" }}>
          {time}
        </span>
      </div>

      {/* Node dot */}
      <div
        style={{
          position: "absolute",
          left: -27,
          top: 6,
          width: isFirst ? 8 : 6,
          height: isFirst ? 8 : 6,
          background: nodeColor,
          borderRadius: isFirst ? 0 : "50%",
          zIndex: 1,
        }}
      />

      {/* Content card */}
      <div
        style={{
          flex: 1,
          background: "var(--color-surface-container)",
          borderLeft: `3px solid ${nodeColor}`,
          padding: "10px 14px",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
          <span
            style={{
              fontFamily: "var(--font-pixel)",
              fontSize: 6,
              color: nodeColor,
              letterSpacing: "0.1em",
              textTransform: "uppercase",
            }}
          >
            {intent.tier}
            {intent.source === "manual" && " / MANUAL"}
            {intent.source === "manual_correction" && " / CORRECTED"}
          </span>
        </div>
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 13,
            lineHeight: 1.6,
            color: "var(--color-text)",
            margin: 0,
          }}
        >
          {intent.content}
        </p>
      </div>
    </div>
  );
}

/* ── Helpers ── */

function formatRelativeTime(iso: string): string {
  try {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "now";
    if (mins < 60) return `${mins}m`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h`;
    return `${Math.floor(hrs / 24)}d`;
  } catch {
    return "";
  }
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}`;
  } catch {
    return "";
  }
}
