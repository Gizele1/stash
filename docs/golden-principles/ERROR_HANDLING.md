# Error Handling

## Rule
Use typed errors with `thiserror` inside library code. Convert to `String` only at the Tauri command boundary. Never panic in production paths; never silently swallow errors in the frontend.

## DO

```rust
// Good: typed error inside library code
#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("database query failed: {0}")]
    Query(#[from] rusqlite::Error),
    #[error("record not found: {0}")]
    NotFound(String),
}

// Good: convert to String only at the command boundary
#[tauri::command]
pub fn task_create(name: String, db: Db<'_>) -> Result<Task, String> {
    db.task_create(&name).map_err(|e| e.to_string())
}
```

```typescript
// Good: surface errors in the UI, don't swallow them
try {
  await api.taskCreate(name, intent);
} catch (e) {
  console.error("Failed to create task:", e);
  setError(String(e));
}
```

## DON'T

```rust
// Bad: bare String as error type inside library code
pub fn task_create(&self, name: &str) -> Result<Task, String> { ... }

// Bad: unwrap() in a production code path
let conn = self.conn.lock().unwrap();

// Bad: silently ignoring errors with .ok() when the result matters
db.branch_update(&b.id, Some(new_status), None, None).ok();
```

```typescript
// Bad: silent catch
try {
  await api.taskCreate(name, intent);
} catch (_) {}
```

## Exceptions
`expect("database mutex poisoned")` is acceptable — mutex poisoning means a thread panicked while holding the lock, which is an unrecoverable programmer error, not a runtime condition.
