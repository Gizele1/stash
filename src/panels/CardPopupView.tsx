import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ContextDetail, ShowCardPayload } from "./types";
import { StatusBadge } from "./components/StatusBadge";

interface CardPopupViewProps {
  payload: ShowCardPayload;
  onDismiss: () => void;
}

export function CardPopupView({ payload, onDismiss }: CardPopupViewProps) {
  const [detail, setDetail] = useState<ContextDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [editingIntent, setEditingIntent] = useState(false);
  const [editText, setEditText] = useState("");
  const [visible, setVisible] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);

  // Fetch context detail
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    invoke<ContextDetail>("get_context_detail", {
      contextId: payload.context_id,
    })
      .then((data) => {
        if (!cancelled) {
          setDetail(data);
          setLoading(false);
        }
      })
      .catch(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [payload.context_id]);

  // Appear animation
  useEffect(() => {
    const timer = setTimeout(() => setVisible(true), 10);
    return () => clearTimeout(timer);
  }, []);

  // Dismiss on Escape
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handleDismiss();
      }
    };
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  });

  // Click outside to dismiss
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (cardRef.current && !cardRef.current.contains(e.target as Node)) {
        handleDismiss();
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  });

  const handleDismiss = useCallback(() => {
    setVisible(false);
    setTimeout(onDismiss, 150);
  }, [onDismiss]);

  const handleJump = useCallback(async () => {
    if (!detail) return;
    const { status } = detail.context;
    if (status === "running" || status === "stuck") {
      await invoke("focus_terminal", { projectDir: detail.context.project_dir });
    } else if (status === "done") {
      await invoke("open_pr_url", { projectDir: detail.context.project_dir });
    }
  }, [detail]);

  const handleIntentCorrection = useCallback(async () => {
    if (!detail?.current_intent || !editText.trim()) return;
    try {
      await invoke("correct_intent", {
        intentId: detail.current_intent.id,
        newContent: editText.trim(),
      });
      setEditingIntent(false);
      // Refresh detail
      const refreshed = await invoke<ContextDetail>("get_context_detail", {
        contextId: payload.context_id,
      });
      setDetail(refreshed);
    } catch {
      // Silently fail
    }
  }, [detail, editText, payload.context_id]);

  const startEdit = useCallback(() => {
    if (detail?.current_intent) {
      setEditText(detail.current_intent.content);
      setEditingIntent(true);
    }
  }, [detail]);

  const formatTime = (iso: string) => {
    const d = new Date(iso);
    return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  return (
    <div
      className={`panels-card-popup ${visible ? "visible" : ""}`}
      ref={cardRef}
      style={{
        left: payload.anchor_position.x,
        top: payload.anchor_position.y,
      }}
    >
      {loading ? (
        <div className="panels-card-loading">Loading...</div>
      ) : detail ? (
        <>
          <div className="panels-card-header">
            <span className="panels-card-name">{detail.context.name}</span>
            <StatusBadge status={detail.context.status} />
          </div>

          <div className="panels-card-body">
            <div className="panels-card-row">
              <span className="panels-card-label">Status</span>
              <span className="panels-card-value">{detail.status}</span>
            </div>

            {detail.current_intent && (
              <div className="panels-card-row">
                <span className="panels-card-label">Intent</span>
                {editingIntent ? (
                  <div className="panels-card-edit">
                    <input
                      type="text"
                      value={editText}
                      onChange={(e) => setEditText(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleIntentCorrection();
                        if (e.key === "Escape") setEditingIntent(false);
                      }}
                      autoFocus
                    />
                    <button type="button" onClick={handleIntentCorrection}>
                      Save
                    </button>
                  </div>
                ) : (
                  <span
                    className="panels-card-value clickable"
                    onClick={startEdit}
                    title="Click to edit"
                  >
                    {detail.current_intent.content}
                  </span>
                )}
              </div>
            )}

            <div className="panels-card-row">
              <span className="panels-card-label">Last Activity</span>
              <span className="panels-card-value">
                {formatTime(detail.context.updated_at)}
              </span>
            </div>
          </div>

          <div className="panels-card-footer">
            <button
              type="button"
              className="panels-card-jump"
              onClick={handleJump}
              disabled={detail.context.status === "parked"}
            >
              Jump
            </button>
          </div>
        </>
      ) : (
        <div className="panels-card-error">Failed to load context</div>
      )}
    </div>
  );
}
