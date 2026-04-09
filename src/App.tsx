import { useState } from "react";
import { DashboardPanel } from "./components/DashboardPanel";
import { ContextList } from "./components/ContextList";
import { api } from "./hooks/useTauri";

type View = "v2" | "v1";

function App() {
  const [view, setView] = useState<View>("v2");

  const handleOpenGraph = (taskId: string) => {
    api.openGraphWindow(taskId).catch(console.error);
  };

  return (
    <div>
      {/* View toggle */}
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid #e5e7eb" }}>
        <TabBtn label="Contexts" active={view === "v2"} onClick={() => setView("v2")} />
        <TabBtn label="Tasks (v1)" active={view === "v1"} onClick={() => setView("v1")} />
      </div>

      {view === "v2" ? (
        <ContextList />
      ) : (
        <DashboardPanel onOpenGraph={handleOpenGraph} />
      )}
    </div>
  );
}

function TabBtn({ label, active, onClick }: { label: string; active: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      style={{
        flex: 1,
        padding: "8px 12px",
        fontSize: 12,
        fontWeight: active ? 600 : 400,
        color: active ? "#3b82f6" : "#6b7280",
        background: "none",
        border: "none",
        borderBottom: active ? "2px solid #3b82f6" : "2px solid transparent",
        cursor: "pointer",
      }}
    >
      {label}
    </button>
  );
}

export default App;
