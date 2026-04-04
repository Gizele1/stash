import { useState } from "react";

interface CreateTaskFormProps {
  onSubmit: (name: string, intent: string) => void;
  onCancel: () => void;
}

export function CreateTaskForm({ onSubmit, onCancel }: CreateTaskFormProps) {
  const [name, setName] = useState("");
  const [intent, setIntent] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (name.trim() && intent.trim()) {
      onSubmit(name.trim(), intent.trim());
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      style={{
        padding: 12,
        marginBottom: 8,
        borderRadius: 8,
        border: "1px solid #3b82f6",
        background: "#eff6ff",
      }}
    >
      <input
        autoFocus
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Task name"
        style={{
          width: "100%",
          padding: "6px 8px",
          marginBottom: 6,
          borderRadius: 4,
          border: "1px solid #d1d5db",
          fontSize: 13,
          boxSizing: "border-box",
        }}
      />
      <input
        value={intent}
        onChange={(e) => setIntent(e.target.value)}
        placeholder="What are you trying to do?"
        style={{
          width: "100%",
          padding: "6px 8px",
          marginBottom: 8,
          borderRadius: 4,
          border: "1px solid #d1d5db",
          fontSize: 13,
          boxSizing: "border-box",
        }}
      />
      <div style={{ display: "flex", gap: 6 }}>
        <button
          type="submit"
          disabled={!name.trim() || !intent.trim()}
          style={{
            fontSize: 12,
            padding: "4px 12px",
            borderRadius: 4,
            border: "none",
            background: "#3b82f6",
            color: "#fff",
            cursor: "pointer",
          }}
        >
          Create
        </button>
        <button
          type="button"
          onClick={onCancel}
          style={{
            fontSize: 12,
            padding: "4px 12px",
            borderRadius: 4,
            border: "1px solid #d1d5db",
            background: "#fff",
            cursor: "pointer",
          }}
        >
          Cancel
        </button>
      </div>
    </form>
  );
}
