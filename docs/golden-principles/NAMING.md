# Naming

## Rule
Follow each language's idiomatic convention without exception. TypeScript: PascalCase for types and React components, camelCase for functions, variables, and hooks. Rust: snake_case for modules, functions, variables, and fields; PascalCase for types, enums, and traits.

## DO

```typescript
// Good: PascalCase for React components and interfaces
export function TaskCard({ data }: Props) { ... }
export interface TaskSummary { id: string; name: string; }

// Good: camelCase for hooks and API functions
export function useTauri() { ... }
export const api = { taskCreate: (...) => invoke(...) };

// Good: camelCase for Tauri invoke arguments (Tauri serializes to snake_case automatically)
invoke<Task>("task_create", { taskId, initialIntent });
```

```rust
// Good: snake_case for modules, functions, fields
pub mod db;
pub fn task_create(name: &str) -> Result<Task, String> { ... }
pub struct Task { pub task_id: String, pub created_at: String }

// Good: PascalCase for types, enums, traits
pub struct Database { ... }
pub enum SessionStatus { Running, Idle, Completed, Error }
pub trait AgentWatcher: Send + Sync { ... }
```

## DON'T

```typescript
// Bad: camelCase for component name (React requires PascalCase for JSX)
export function taskCard() { ... }

// Bad: PascalCase for hook name
export function UseTauri() { ... }
```

```rust
// Bad: camelCase in Rust — cargo clippy will flag this
pub fn taskCreate() { ... }
pub struct sessionStatus { ... }
```

## Exceptions
Tauri command names use snake_case in Rust (`task_create`) and are invoked as snake_case strings from the frontend (`invoke("task_create", ...)`). The `api.*` wrapper in `useTauri.ts` exposes them as camelCase (`api.taskCreate`) — this adapter is intentional.
