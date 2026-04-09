# Imports

## Rule
Import only from layers below you in the hierarchy. Use relative paths within a layer. Never reach across layers laterally or upward.

## DO

```typescript
// Good: component imports from hooks and types (layers below)
import { api } from "../hooks/useTauri";
import type { Task } from "../types/models";
```

```rust
// Good: commands imports only from db and events (layers below)
use crate::db::Database;
use crate::events::EventAggregator;
```

## DON'T

```typescript
// Bad: hook reaching into components (upward import)
import { TaskCard } from "../components/TaskCard";

// Bad: component importing from a sibling component (lateral import)
import { CreateTaskForm } from "./CreateTaskForm"; // inside DashboardPanel
```

```rust
// Bad: db importing from commands (upward import)
use crate::commands::task_create;

// Bad: watcher importing from db (watcher is a leaf)
use crate::db::Database;
```

## Exceptions
`lib.rs` is the composition root and may import from all modules — it exists solely to wire everything together.
