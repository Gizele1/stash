# Architecture

Stash is a Tauri v2 desktop application. The React frontend communicates with the Rust backend exclusively through Tauri's typed IPC command layer. No shared filesystem state exists between layers at runtime.

## Domain Map

```
┌────────────────────────────────────────────┐
│  Frontend  src/                            │
│  React 19 + TypeScript + Vite 6            │
│                                            │
│  App.tsx → components → hooks → types      │
└─────────────────┬──────────────────────────┘
                  │  Tauri IPC (invoke / emit)
┌─────────────────▼──────────────────────────┐
│  Backend  src-tauri/src/                   │
│  Rust 2021 + Tauri 2                       │
│                                            │
│  commands ──┐                              │
│  capture    ├──→ db/ (rusqlite + SQLite)   │
│  watcher    │    events/                   │
│  intent  ───┘                              │
└─────────────────┬──────────────────────────┘
                  │  Filesystem
┌─────────────────▼──────────────────────────┐
│  Persistence                               │
│  SQLite  →  $APP_DATA_DIR/stash.db         │
│  Sessions → ~/.claude/projects/**/*.jsonl  │
└────────────────────────────────────────────┘
```

## Key Design Decisions

- **Append-only intent history** — intents are never mutated; a new version row is inserted on every change. This makes the intent graph fully reproducible from the database alone.
- **Auto-capture loop** — a background Rust thread polls `~/.claude/projects/` every 5 seconds; no user interaction required to track sessions.
- **Single IPC surface** — `src/hooks/useTauri.ts` is the only file that calls `invoke()`. All components use `api.*` from that module. This keeps the Tauri API surface auditable in one place.
- **Partial capture tolerance** — environment snapshots time out after 2 s and return whatever they collected; the UI degrades gracefully on slow systems.

## Layer Enforcement

Full rules: `docs/architecture/LAYERS.md`

Frontend boundary test: `tests/architecture/boundary.test.ts` — run with `npm test`

Backend boundary test: `src-tauri/tests/architecture.rs` — run with `cargo test`
