import { ContextList } from "./components/ContextList";

function App() {
  return (
    <div style={{ minHeight: "100vh", display: "flex", flexDirection: "column" }}>
      {/* Arcade Header */}
      <header
        style={{
          background: "#000",
          borderBottom: "2px solid var(--color-primary-light)",
          padding: "0 16px",
          height: 40,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          flexShrink: 0,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span
            style={{
              fontFamily: "var(--font-pixel)",
              fontSize: 11,
              color: "var(--color-primary-light)",
              letterSpacing: "0.1em",
            }}
          >
            STASH_V2
          </span>
          <span
            style={{
              fontFamily: "var(--font-pixel)",
              fontSize: 7,
              color: "var(--color-text-muted)",
              letterSpacing: "0.05em",
            }}
          >
            .OS
          </span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          <span
            style={{
              fontFamily: "var(--font-pixel)",
              fontSize: 8,
              color: "var(--color-secondary)",
            }}
          >
            CONTEXTS
          </span>
          <span
            className="material-symbols-outlined"
            style={{ color: "var(--color-primary-light)", fontSize: 18, cursor: "pointer" }}
          >
            settings
          </span>
        </div>
      </header>

      {/* Main Content */}
      <main style={{ flex: 1, overflow: "auto" }}>
        <ContextList />
      </main>
    </div>
  );
}

export default App;
