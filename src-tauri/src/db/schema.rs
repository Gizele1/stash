use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('active', 'parked')),
            current_intent_id TEXT,
            created_at TEXT NOT NULL,
            parked_at TEXT,
            last_active_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS task_dependencies (
            from_task_id TEXT NOT NULL REFERENCES tasks(id),
            to_task_id TEXT NOT NULL REFERENCES tasks(id),
            PRIMARY KEY (from_task_id, to_task_id)
        );

        CREATE TABLE IF NOT EXISTS intent_snapshots (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id),
            version INTEGER NOT NULL,
            statement TEXT NOT NULL,
            trigger_type TEXT NOT NULL CHECK(trigger_type IN ('initial', 'refinement', 'drift_response', 'auto_inferred')),
            reason TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS agent_branches (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id),
            agent_platform TEXT NOT NULL,
            platform_color TEXT NOT NULL,
            forked_from_intent_id TEXT NOT NULL REFERENCES intent_snapshots(id),
            status TEXT NOT NULL CHECK(status IN ('running', 'completed', 'error', 'abandoned')),
            progress REAL CHECK(progress IS NULL OR (progress >= 0.0 AND progress <= 1.0)),
            output_ref TEXT,
            source_type TEXT NOT NULL CHECK(source_type IN ('auto', 'manual')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS drift_markers (
            id TEXT PRIMARY KEY,
            branch_id TEXT NOT NULL REFERENCES agent_branches(id),
            summary TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS resume_notes (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) UNIQUE,
            content TEXT NOT NULL,
            source TEXT NOT NULL CHECK(source IN ('auto', 'manual')),
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS environment_snapshots (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id),
            git_branch TEXT,
            git_status TEXT,
            git_diff_summary TEXT,
            active_files TEXT,
            terminal_last_output TEXT,
            window_focus TEXT,
            agent_states TEXT,
            captured_at TEXT NOT NULL,
            completeness TEXT NOT NULL CHECK(completeness IN ('full', 'partial'))
        );

        CREATE TABLE IF NOT EXISTS agent_events (
            id TEXT PRIMARY KEY,
            branch_id TEXT NOT NULL REFERENCES agent_branches(id),
            event_type TEXT NOT NULL CHECK(event_type IN ('progress_update', 'completed', 'error', 'commit_detected')),
            summary TEXT,
            metadata TEXT,
            created_at TEXT NOT NULL,
            briefing_id TEXT
        );

        CREATE TABLE IF NOT EXISTS review_logs (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id),
            branch_id TEXT NOT NULL REFERENCES agent_branches(id),
            started_at TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL,
            outcome TEXT NOT NULL CHECK(outcome IN ('approved', 'rejected', 'rejected_partial'))
        );

        CREATE TABLE IF NOT EXISTS briefings (
            id TEXT PRIMARY KEY,
            generated_at TEXT NOT NULL,
            read_at TEXT,
            items TEXT NOT NULL
        );

        -- ── v2 tables for Brain module ──

        CREATE TABLE IF NOT EXISTS contexts (
            id TEXT PRIMARY KEY,
            project_key TEXT NOT NULL UNIQUE,
            project_dir TEXT NOT NULL,
            name TEXT NOT NULL,
            manual_assignment_required INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'done', 'stuck', 'parked')),
            status_override_until TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS raw_prompts (
            id TEXT PRIMARY KEY,
            context_id TEXT NOT NULL REFERENCES contexts(id),
            session_path TEXT NOT NULL,
            message_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            captured_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS prompt_consumptions (
            prompt_id TEXT PRIMARY KEY REFERENCES raw_prompts(id),
            processed_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS intents_v2 (
            id TEXT PRIMARY KEY,
            context_id TEXT NOT NULL REFERENCES contexts(id),
            tier TEXT NOT NULL CHECK(tier IN ('narrative', 'summary', 'label')),
            content TEXT NOT NULL,
            source TEXT NOT NULL DEFAULT 'auto' CHECK(source IN ('auto', 'manual', 'manual_correction', 'compression')),
            created_at TEXT NOT NULL,
            archived INTEGER NOT NULL DEFAULT 0,
            archived_at TEXT,
            compressed_from TEXT
        );

        -- ── Config table (used by Platform module for pet position, etc.) ──

        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- ── Intent compression sources (for v1 intents compatibility) ──

        CREATE TABLE IF NOT EXISTS intent_compression_sources (
            intent_id TEXT NOT NULL,
            source_intent_id TEXT NOT NULL,
            PRIMARY KEY (intent_id, source_intent_id)
        );

        CREATE INDEX IF NOT EXISTS idx_intent_snapshots_task ON intent_snapshots(task_id, version);
        CREATE INDEX IF NOT EXISTS idx_agent_branches_task ON agent_branches(task_id);
        CREATE INDEX IF NOT EXISTS idx_agent_events_branch ON agent_events(branch_id);
        CREATE INDEX IF NOT EXISTS idx_agent_events_briefing ON agent_events(briefing_id);
        CREATE INDEX IF NOT EXISTS idx_review_logs_task ON review_logs(task_id);
        CREATE INDEX IF NOT EXISTS idx_environment_snapshots_task ON environment_snapshots(task_id, captured_at);
        CREATE INDEX IF NOT EXISTS idx_raw_prompts_context ON raw_prompts(context_id, captured_at);
        CREATE INDEX IF NOT EXISTS idx_intents_v2_context ON intents_v2(context_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_intents_v2_archived ON intents_v2(archived, created_at);
        ",
    )?;
    Ok(())
}
