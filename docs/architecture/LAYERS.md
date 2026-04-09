# Architecture Layers

Dependency flows **downward only**. Importing upward is a violation and will fail CI.

## Frontend Layer Hierarchy

```
[App.tsx]
  └── [components/]    can import: hooks, types
        └── [hooks/]   can import: types
              └── [types/]   (leaf — no internal imports)
```

### Allowed Imports

| Layer          | May import from               | May NOT import from                          |
|----------------|-------------------------------|----------------------------------------------|
| `App.tsx`      | components                    | hooks directly, types directly               |
| `components/`  | hooks, types                  | other components (lateral), App              |
| `hooks/`       | types                         | components, App                              |
| `types/`       | nothing (leaf)                | everything                                   |
| `graph/`       | components, hooks, types      | App (no upward imports from components/hooks)|
| `pet/`         | components, hooks, types      | App (no upward imports from components/hooks)|

### Notes on Window Entry Points

`src/graph/` and `src/pet/` are **window entry points** — each directory contains the React root for a separate Tauri window. They may import from `src/components/`, `src/hooks/`, and `src/types/`.

**Upward import rule:** `src/components/` and `src/hooks/` may NOT import from `src/graph/` or `src/pet/`. Dependency flows downward only.

### Enforcement

- **Test:** `npm test -- tests/architecture/boundary.test.ts`
- **Lint:** ESLint `no-restricted-imports` rules (see `eslint.config.mjs`)
- **Error format:** `VIOLATION: {file}:{line} imports {target} — {layer} cannot import {target_layer}. See docs/architecture/LAYERS.md`

---

## Backend Layer Hierarchy

```
[lib.rs / main.rs]   Composition root — wires all modules together
  ├── [commands/]    can import: db, events
  ├── [capture/]     leaf — external crates only (tokio, git2)
  ├── [watcher/]     leaf — external crates only (dirs, serde_json)
  ├── [intent/]      leaf
  ├── [events/]      leaf
  └── [db/]          leaf
        ├── schema.rs
        ├── models.rs
        └── queries.rs
```

### Allowed Internal (`crate::`) Imports

| Module      | May import from (`crate::`) | May NOT import from (`crate::`)       |
|-------------|------------------------------|---------------------------------------|
| `commands`  | db, events                   | capture, watcher, intent              |
| `capture`   | nothing (leaf)               | everything                            |
| `watcher`   | nothing (leaf)               | everything                            |
| `intent`    | nothing (leaf)               | everything                            |
| `events`    | nothing (leaf)               | everything                            |
| `db`        | nothing (leaf)               | everything                            |

`lib.rs` is the **composition root** — it is the only file exempt from these rules and may import from all modules.

### Enforcement

- **Test:** `cargo test -- architecture` (runs `src-tauri/tests/architecture.rs`)
- **Visibility:** Use `pub(crate)` instead of `pub` where possible to prevent accidental cross-module access
- **Error format:** `VIOLATION: src/{module}/mod.rs:{line} uses crate::{target} — {module} cannot import {target}. See docs/architecture/LAYERS.md`

---

## Remediation Guide

If you get a layer violation:

1. Read the error — it tells you which rule was broken
2. Ask: is this dependency truly necessary, or can the logic be moved to a shared leaf?
3. If necessary: move shared logic down to a leaf layer (e.g., `types/`, `db::models`)
4. If a violation must remain temporarily, add it to `tests/architecture/known-violations.json` with a reason and target removal date
5. **KNOWN_VIOLATIONS can only shrink** — never add new ones without a removal date commitment

## Known Violations Baseline

File: `tests/architecture/known-violations.json`

Currently: **0 violations** — the baseline is clean. Any new violation will fail CI immediately.
