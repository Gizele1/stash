import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ContextSelector } from "./components/ContextSelector";

interface ManualInputViewProps {
  onDismiss: () => void;
}

export function ManualInputView({ onDismiss }: ManualInputViewProps) {
  const [contextId, setContextId] = useState("");
  const [content, setContent] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Auto-focus on the text input
  useEffect(() => {
    const timer = setTimeout(() => inputRef.current?.focus(), 50);
    return () => clearTimeout(timer);
  }, []);

  // Escape to dismiss
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onDismiss();
    };
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onDismiss]);

  const handleSubmit = useCallback(async () => {
    if (!contextId || !content.trim()) return;
    setSubmitting(true);
    setError(null);
    try {
      await invoke("submit_manual_intent", {
        contextId,
        content: content.trim(),
      });
      setContent("");
      onDismiss();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  }, [contextId, content, onDismiss]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
      }
    },
    [handleSubmit],
  );

  return (
    <div className="panels-manual-input">
      <div className="panels-input-header">
        <h3>Manual Intent</h3>
      </div>

      <div className="panels-input-body">
        <ContextSelector value={contextId} onChange={setContextId} />

        <textarea
          ref={inputRef}
          className="panels-input-text"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Describe what you're working on..."
          rows={3}
          disabled={submitting}
        />

        {error && <div className="panels-input-error">{error}</div>}
      </div>

      <div className="panels-input-footer">
        <button
          type="button"
          className="panels-input-cancel"
          onClick={onDismiss}
          disabled={submitting}
        >
          Cancel
        </button>
        <button
          type="button"
          className="panels-input-submit"
          onClick={handleSubmit}
          disabled={!contextId || !content.trim() || submitting}
        >
          {submitting ? "Submitting..." : "Submit"}
        </button>
      </div>
    </div>
  );
}
