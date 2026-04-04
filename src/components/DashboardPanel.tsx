import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { TaskCard as TaskCardData } from "../types/models";
import { api } from "../hooks/useTauri";
import { TaskCard } from "./TaskCard";
import { CreateTaskForm } from "./CreateTaskForm";

interface Props {
  onOpenGraph?: (taskId: string) => void;
}

export function DashboardPanel({ onOpenGraph }: Props) {
  const [cards, setCards] = useState<TaskCardData[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [unreviewedCount, setUnreviewedCount] = useState(0);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const tasks = await api.taskList();
      const cardPromises = tasks.map((t) => api.taskGetCard(t.id));
      const results = await Promise.all(cardPromises);
      setCards(results);
      const count = await api.getUnreviewedBranchCount();
      setUnreviewedCount(count);
    } catch (e) {
      console.error("Failed to load tasks:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    // Listen for real-time capture updates from backend
    const unlisten = listen("stash://capture-update", () => {
      refresh();
    });
    return () => {
      clearInterval(interval);
      unlisten.then((fn) => fn());
    };
  }, [refresh]);

  const handleCreate = async (name: string, intent: string) => {
    try {
      await api.taskCreate(name, intent);
      setShowCreate(false);
      refresh();
    } catch (e) {
      console.error("Failed to create task:", e);
    }
  };

  const handlePark = async (taskId: string) => {
    try {
      await api.taskPark(taskId);
      refresh();
    } catch (e) {
      console.error("Failed to park task:", e);
    }
  };

  const handleSelect = (taskId: string) => {
    if (onOpenGraph) {
      onOpenGraph(taskId);
    }
  };

  const activeTasks = cards.filter((c) => c.task.status === "active");
  const parkedTasks = cards.filter((c) => c.task.status === "parked");

  return (
    <div style={{ padding: 16, maxWidth: 400, fontFamily: "system-ui, -apple-system, sans-serif" }}>
      {/* Header */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 12,
        }}
      >
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 700 }}>Stash</h2>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          {unreviewedCount > 0 && (
            <span
              style={{
                fontSize: 11,
                background: "#ef4444",
                color: "#fff",
                padding: "2px 7px",
                borderRadius: 10,
                fontWeight: 600,
              }}
            >
              {unreviewedCount}
            </span>
          )}
          <button
            onClick={() => setShowCreate(!showCreate)}
            style={{
              fontSize: 18,
              lineHeight: 1,
              background: "none",
              border: "none",
              cursor: "pointer",
              color: "#3b82f6",
              padding: 0,
            }}
            title="New task"
          >
            +
          </button>
        </div>
      </div>

      {/* Create form */}
      {showCreate && (
        <CreateTaskForm
          onSubmit={handleCreate}
          onCancel={() => setShowCreate(false)}
        />
      )}

      {loading ? (
        <p style={{ color: "#9ca3af", fontSize: 13 }}>Loading...</p>
      ) : cards.length === 0 ? (
        <p style={{ color: "#9ca3af", fontSize: 13 }}>
          No tasks yet. Click + to create one.
        </p>
      ) : (
        <>
          {/* Active tasks */}
          {activeTasks.length > 0 && (
            <div style={{ marginBottom: 12 }}>
              <h3 style={{ fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "0 0 6px", letterSpacing: 0.5 }}>
                Active ({activeTasks.length})
              </h3>
              {activeTasks.map((c) => (
                <TaskCard
                  key={c.task.id}
                  data={c}
                  onSelect={handleSelect}
                  onPark={handlePark}
                />
              ))}
            </div>
          )}

          {/* Parked tasks */}
          {parkedTasks.length > 0 && (
            <div>
              <h3 style={{ fontSize: 11, color: "#6b7280", textTransform: "uppercase", margin: "0 0 6px", letterSpacing: 0.5 }}>
                Parked ({parkedTasks.length})
              </h3>
              {parkedTasks.map((c) => (
                <TaskCard
                  key={c.task.id}
                  data={c}
                  onSelect={handleSelect}
                  onPark={handlePark}
                />
              ))}
            </div>
          )}
        </>
      )}

      {/* Park all shortcut */}
      {activeTasks.length > 1 && (
        <button
          onClick={async () => {
            await api.parkAllTasks();
            refresh();
          }}
          style={{
            marginTop: 8,
            fontSize: 11,
            color: "#6b7280",
            background: "none",
            border: "1px solid #e5e7eb",
            borderRadius: 4,
            padding: "4px 10px",
            cursor: "pointer",
            width: "100%",
          }}
        >
          Park all tasks
        </button>
      )}
    </div>
  );
}
