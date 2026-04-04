import type { TaskCard as TaskCardData } from "../types/models";

const STATUS_COLORS: Record<string, string> = {
  running: "#3b82f6",
  completed: "#22c55e",
  error: "#ef4444",
  abandoned: "#6b7280",
};

interface TaskCardProps {
  data: TaskCardData;
  onSelect: (taskId: string) => void;
  onPark: (taskId: string) => void;
}

export function TaskCard({ data, onSelect, onPark }: TaskCardProps) {
  const { task, current_intent, branches, resume_note, has_drift } = data;

  return (
    <div
      onClick={() => onSelect(task.id)}
      style={{
        padding: 12,
        marginBottom: 8,
        borderRadius: 8,
        border: has_drift ? "2px solid #f59e0b" : "1px solid #e5e7eb",
        background: task.status === "active" ? "#fff" : "#f9fafb",
        cursor: "pointer",
        transition: "box-shadow 0.15s",
      }}
      onMouseEnter={(e) =>
        (e.currentTarget.style.boxShadow = "0 2px 8px rgba(0,0,0,0.08)")
      }
      onMouseLeave={(e) => (e.currentTarget.style.boxShadow = "none")}
    >
      {/* Header row */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 6,
        }}
      >
        <span style={{ fontWeight: 600, fontSize: 14 }}>{task.name}</span>
        <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
          {has_drift && (
            <span
              style={{
                fontSize: 10,
                background: "#fef3c7",
                color: "#92400e",
                padding: "2px 6px",
                borderRadius: 4,
                fontWeight: 500,
              }}
            >
              DRIFT
            </span>
          )}
          <span
            style={{
              fontSize: 11,
              color: "#6b7280",
              textTransform: "uppercase",
            }}
          >
            {task.status}
          </span>
        </div>
      </div>

      {/* Intent statement */}
      {current_intent && (
        <p
          style={{
            fontSize: 12,
            color: "#374151",
            margin: "0 0 8px",
            lineHeight: 1.4,
          }}
        >
          {current_intent.statement}
        </p>
      )}

      {/* Agent branch dots */}
      {branches.length > 0 && (
        <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 6 }}>
          {branches.map((b) => (
            <div
              key={b.id}
              title={`${b.agent_platform} — ${b.status}`}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 3,
                fontSize: 11,
                color: "#6b7280",
              }}
            >
              <span
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: STATUS_COLORS[b.status] || "#9ca3af",
                  display: "inline-block",
                }}
              />
              {b.agent_platform}
              {b.progress != null && (
                <span style={{ fontSize: 10, color: "#9ca3af" }}>
                  {Math.round(b.progress * 100)}%
                </span>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Resume note preview */}
      {resume_note && (
        <p
          style={{
            fontSize: 11,
            color: "#9ca3af",
            margin: 0,
            fontStyle: "italic",
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
          }}
        >
          {resume_note.content}
        </p>
      )}

      {/* Park button */}
      {task.status === "active" && (
        <button
          onClick={(e) => {
            e.stopPropagation();
            onPark(task.id);
          }}
          style={{
            marginTop: 6,
            fontSize: 11,
            color: "#6b7280",
            background: "none",
            border: "1px solid #e5e7eb",
            borderRadius: 4,
            padding: "2px 8px",
            cursor: "pointer",
          }}
        >
          Park
        </button>
      )}
    </div>
  );
}
