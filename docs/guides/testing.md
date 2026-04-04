# Testing Guide

## Frontend Tests (Vitest)

```sh
npm test          # Run once
npm run test:watch  # Watch mode during development
```

Test files live next to source files (`src/**/*.test.ts`) or in `tests/` for cross-cutting concerns.

### Architecture Boundary Test

`tests/architecture/boundary.test.ts` scans all `src/` files and validates that imports respect the layer hierarchy defined in `docs/architecture/LAYERS.md`.

If a new violation appears, the test fails with:
```
VIOLATION: src/hooks/useTauri.ts:5 imports ../components/TaskCard — hooks cannot import components. See docs/architecture/LAYERS.md
```

To grandfather a temporary violation, add it to `tests/architecture/known-violations.json` with a reason. The count can only shrink.

## Backend Tests (cargo test)

```sh
cd src-tauri
cargo test                    # All tests
cargo test -- architecture    # Architecture boundary test only
cargo test -- db              # DB layer tests only
```

The `db::Database::in_memory()` constructor creates a fresh in-memory SQLite instance for each test — use it for all DB tests to avoid test pollution.

### Example DB test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn test_task_create_and_retrieve() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("my-project").unwrap();
        assert_eq!(task.name, "my-project");
        assert_eq!(task.status, "active");
    }
}
```

## What to Test

| Layer | Test focus |
|-------|-----------|
| `db/` | All CRUD operations using `in_memory()` |
| `intent/` | Extraction logic with sample inputs |
| `watcher/` | Session parsing with fixture JSONL files |
| `components/` | Render tests for complex state logic |
| `tests/architecture/` | Layer boundary ratchet (automated) |
