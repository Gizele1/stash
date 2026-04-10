# Stash — Agent Orientation Map

> Tauri v2 desktop app: watches Claude Code sessions and surfaces them as tasks with evolving intent, agent branches, and drift markers — a "git stash for your brain".

## Stack

| Layer     | Tech                           |
|-----------|--------------------------------|
| Frontend  | React 19 + TypeScript + Vite 6 |
| Backend   | Rust 2021 + Tauri 2            |
| Database  | SQLite via rusqlite (bundled)  |
| Graph UI  | @xyflow/react + dagre          |

## Architecture Layers

Dependency flows **downward only**. Never import upward.

### Frontend (`src/`)

```
App.tsx
└── components/   UI components            (imports: hooks, types)
    └── hooks/    Tauri IPC bridge         (imports: types)
        └── types/ Pure TS interfaces      (leaf — no imports)
```

### Backend (`src-tauri/src/`)

```
lib.rs            Composition root         (wires everything)
├── commands/     Tauri IPC handlers       (imports: db, events)
├── capture/      Environment snapshot     (leaf — external crates only)
├── watcher/      Claude Code session poll (leaf — external crates only)
├── intent/       Intent extraction rules  (leaf)
├── events/       Event aggregator         (leaf)
└── db/           SQLite layer             (leaf)
    ├── schema.rs
    ├── models.rs
    └── queries.rs
```

## Key Conventions

- Tauri IPC is the **only** bridge between frontend and backend — all calls go through `src/hooks/useTauri.ts`
- All DB access uses `db::Database` — never open `rusqlite::Connection` directly
- Intent is **append-only** — create new versions, never mutate existing rows
- See `docs/golden-principles/` for DO/DON'T code examples

## Commands

```sh
# Dev
npm run dev           # Vite frontend only
npm run tauri         # Full Tauri app (frontend + backend)

# Build
npm run build         # Frontend (Vite)
cd src-tauri && cargo build --release

# Test
npm test              # Vitest (includes frontend architecture boundary test)
cd src-tauri && cargo test

# Lint / Typecheck
npm run lint          # ESLint
npm run typecheck     # tsc --noEmit
cd src-tauri && cargo clippy

# Garbage collection
npm run gc            # Entropy scan (doc drift, architecture violations)
```

## Documentation Map

```
ARCHITECTURE.md                    Top-level domain map
docs/
├── architecture/
│   └── LAYERS.md                  Layer rules + enforcement details
├── golden-principles/
│   ├── IMPORTS.md                 Import ordering and path rules
│   ├── ERROR_HANDLING.md          Error propagation patterns
│   └── NAMING.md                  File and export naming conventions
├── SECURITY.md                    Threat model, secrets, data handling
└── guides/
    ├── setup.md                   Dev environment setup
    └── testing.md                 Testing guide
```

## Where to Look First

| Task                          | Start here                                       |
|-------------------------------|--------------------------------------------------|
| Architecture overview         | ARCHITECTURE.md                                  |
| Layer rules + enforcement     | docs/architecture/LAYERS.md                      |
| Add a Tauri command           | src-tauri/src/commands/mod.rs                    |
| Add a DB query                | src-tauri/src/db/queries.rs                      |
| Add a React component         | src/components/                                  |
| Understand data models        | src/types/models.ts + src-tauri/src/db/models.rs |
| Debug auto-capture            | src-tauri/src/watcher/ + src-tauri/src/lib.rs    |

## Constraints (Machine-Readable)

- MUST: All frontend→backend calls go through `src/hooks/useTauri.ts`
- MUST: All DB access uses `db::Database` — never raw `rusqlite::Connection`
- MUST NOT: `db/` may not import from `commands/`, `capture/`, or `watcher/`
- MUST NOT: Frontend `components/` may not import from sibling components
- PREFER: `thiserror` for Rust error types — not bare `String` in library code
- VERIFY: `npm run typecheck && npm test && cargo clippy && cargo test` before PR

## Lash (Auto-triggered Multi-Agent Build Orchestrator)

When ALL of the following conditions are met:
1. Spec artifact exists: `specs/spec.json` OR `specs/spec/index.json` (design is complete)
2. Discover artifact exists: `specs/discover.json` OR `specs/discover/index.json` (requirements are locked)
3. User intent involves building, implementing, or coding the designed system

→ Invoke the installed Lash build prompt to orchestrate a multi-agent parallel build:
  - Claude Code: `/lash-build`
  - Codex: `/prompts:lash-build`

Lash treats each AI coding platform (Claude Code, Codex, OpenCode) as a Worker agent.
Lash auto-detects single-file vs split-directory format for spec and discover artifacts.

NoPilot schemas and workflow definition are in the npm package.
Run `nopilot paths` to locate them.
