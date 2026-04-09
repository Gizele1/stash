import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { StashConfig } from "./types";

interface SettingsViewProps {
  onDismiss: () => void;
}

export function SettingsView({ onDismiss }: SettingsViewProps) {
  const [config, setConfig] = useState<StashConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    invoke<StashConfig>("get_config")
      .then((data) => {
        setConfig(data);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  // Escape to dismiss
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onDismiss();
    };
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onDismiss]);

  const updateField = useCallback(
    <K extends keyof StashConfig>(key: K, value: StashConfig[K]) => {
      setConfig((prev) => (prev ? { ...prev, [key]: value } : prev));
      setDirty(true);
    },
    [],
  );

  const handleSave = useCallback(async () => {
    if (!config) return;
    setSaving(true);
    try {
      await invoke("set_config", { config });
      setDirty(false);
    } catch {
      // Save failed silently
    } finally {
      setSaving(false);
    }
  }, [config]);

  if (loading) {
    return <div className="panels-settings-loading">Loading settings...</div>;
  }

  if (!config) {
    return <div className="panels-settings-error">Failed to load settings</div>;
  }

  return (
    <div className="panels-settings">
      <div className="panels-settings-header">
        <h2>Settings</h2>
        <button type="button" className="panels-settings-close" onClick={onDismiss}>
          Close
        </button>
      </div>

      <div className="panels-settings-body">
        <section className="panels-settings-section">
          <h3>LLM Mode</h3>
          <div className="panels-settings-toggle-group">
            {(["local", "hybrid", "cloud"] as const).map((mode) => (
              <button
                key={mode}
                type="button"
                className={`panels-settings-toggle ${config.llm_mode === mode ? "active" : ""}`}
                onClick={() => updateField("llm_mode", mode)}
              >
                {mode.charAt(0).toUpperCase() + mode.slice(1)}
              </button>
            ))}
          </div>
        </section>

        {(config.llm_mode === "local" || config.llm_mode === "hybrid") && (
          <section className="panels-settings-section">
            <h3>Ollama URL</h3>
            <input
              type="text"
              className="panels-settings-input"
              value={config.ollama_url}
              onChange={(e) => updateField("ollama_url", e.target.value)}
              placeholder="http://localhost:11434"
            />
          </section>
        )}

        {(config.llm_mode === "cloud" || config.llm_mode === "hybrid") && (
          <section className="panels-settings-section">
            <h3>Cloud API Key</h3>
            <input
              type="password"
              className="panels-settings-input"
              value={config.cloud_api_key}
              onChange={(e) => updateField("cloud_api_key", e.target.value)}
              placeholder="sk-..."
            />
          </section>
        )}

        <section className="panels-settings-section">
          <h3>Pet Position</h3>
          <div className="panels-settings-position">
            <span>
              x: {config.pet_position.x}, y: {config.pet_position.y}
            </span>
          </div>
        </section>

        <section className="panels-settings-section">
          <h3>Hotkey</h3>
          <div className="panels-settings-hotkey">
            <code>{config.hotkey}</code>
          </div>
        </section>
      </div>

      <div className="panels-settings-footer">
        <button
          type="button"
          className="panels-settings-save"
          onClick={handleSave}
          disabled={!dirty || saving}
        >
          {saving ? "Saving..." : "Save"}
        </button>
      </div>
    </div>
  );
}
