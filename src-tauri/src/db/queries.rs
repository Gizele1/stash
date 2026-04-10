use super::models::*;
use super::Database;
use rusqlite::params;
use uuid::Uuid;

fn new_id() -> String {
    Uuid::now_v7().to_string()
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn default_config() -> AppConfig {
    AppConfig {
        llm_mode: "local".to_string(),
        local_model: "qwen2.5:7b".to_string(),
        cloud_model: None,
        cloud_endpoint: None,
    }
}

// ── Task CRUD ──

impl Database {
    pub fn task_create(&self, name: &str) -> Result<Task, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO tasks (id, name, status, current_intent_id, created_at, parked_at, last_active_at)
             VALUES (?1, ?2, 'active', NULL, ?3, NULL, ?3)",
            params![id, name, ts],
        ).map_err(|e| e.to_string())?;
        Ok(Task { id, name: name.to_string(), status: "active".to_string(), current_intent_id: None, created_at: ts.clone(), parked_at: None, last_active_at: ts })
    }

    pub fn task_get(&self, id: &str) -> Result<Task, String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, name, status, current_intent_id, created_at, parked_at, last_active_at FROM tasks WHERE id = ?1",
            params![id],
            |row| Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                current_intent_id: row.get(3)?,
                created_at: row.get(4)?,
                parked_at: row.get(5)?,
                last_active_at: row.get(6)?,
            }),
        ).map_err(|e| format!("TASK_NOT_FOUND: {e}"))
    }

    pub fn task_list(&self, status_filter: Option<&str>) -> Result<Vec<Task>, String> {
        let conn = self.conn();
        let mut tasks = Vec::new();
        if let Some(status) = status_filter {
            let mut stmt = conn.prepare(
                "SELECT id, name, status, current_intent_id, created_at, parked_at, last_active_at FROM tasks WHERE status = ?1 ORDER BY last_active_at DESC"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![status], |row| Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                current_intent_id: row.get(3)?,
                created_at: row.get(4)?,
                parked_at: row.get(5)?,
                last_active_at: row.get(6)?,
            })).map_err(|e| e.to_string())?;
            for r in rows { tasks.push(r.map_err(|e| e.to_string())?); }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, name, status, current_intent_id, created_at, parked_at, last_active_at FROM tasks ORDER BY last_active_at DESC"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |row| Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                status: row.get(2)?,
                current_intent_id: row.get(3)?,
                created_at: row.get(4)?,
                parked_at: row.get(5)?,
                last_active_at: row.get(6)?,
            })).map_err(|e| e.to_string())?;
            for r in rows { tasks.push(r.map_err(|e| e.to_string())?); }
        }
        Ok(tasks)
    }

    pub fn task_update_status(&self, id: &str, status: &str) -> Result<Task, String> {
        {
            let conn = self.conn();
            let ts = now();
            let parked_at: Option<String> = if status == "parked" { Some(ts.clone()) } else { None };
            conn.execute(
                "UPDATE tasks SET status = ?1, parked_at = ?2, last_active_at = ?3 WHERE id = ?4",
                params![status, parked_at, ts, id],
            ).map_err(|e| e.to_string())?;
        }
        self.task_get(id)
    }

    pub fn task_set_dependency(&self, from_id: &str, to_id: &str) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR IGNORE INTO task_dependencies (from_task_id, to_task_id) VALUES (?1, ?2)",
            params![from_id, to_id],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn task_get_dependencies(&self, task_id: &str) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT to_task_id FROM task_dependencies WHERE from_task_id = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![task_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        let mut deps = Vec::new();
        for r in rows { deps.push(r.map_err(|e| e.to_string())?); }
        Ok(deps)
    }

    // ── IntentSnapshot CRUD ──

    pub fn intent_create(&self, task_id: &str, statement: &str, trigger_type: &str, reason: Option<&str>) -> Result<IntentSnapshot, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        let version: i64 = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM intent_snapshots WHERE task_id = ?1",
            params![task_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT INTO intent_snapshots (id, task_id, version, statement, trigger_type, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, task_id, version, statement, trigger_type, reason, ts],
        ).map_err(|e| e.to_string())?;

        conn.execute(
            "UPDATE tasks SET current_intent_id = ?1, last_active_at = ?2 WHERE id = ?3",
            params![id, ts, task_id],
        ).map_err(|e| e.to_string())?;

        Ok(IntentSnapshot { id, task_id: task_id.to_string(), version, statement: statement.to_string(), trigger_type: trigger_type.to_string(), reason: reason.map(String::from), created_at: ts })
    }

    pub fn intent_list(&self, task_id: &str) -> Result<Vec<IntentSnapshot>, String> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, version, statement, trigger_type, reason, created_at FROM intent_snapshots WHERE task_id = ?1 ORDER BY version ASC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![task_id], |row| Ok(IntentSnapshot {
            id: row.get(0)?, task_id: row.get(1)?, version: row.get(2)?,
            statement: row.get(3)?, trigger_type: row.get(4)?, reason: row.get(5)?, created_at: row.get(6)?,
        })).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }

    pub fn intent_get_current(&self, task_id: &str) -> Result<Option<IntentSnapshot>, String> {
        let conn = self.conn();
        let result = conn.query_row(
            "SELECT i.id, i.task_id, i.version, i.statement, i.trigger_type, i.reason, i.created_at
             FROM intent_snapshots i JOIN tasks t ON t.current_intent_id = i.id WHERE t.id = ?1",
            params![task_id],
            |row| Ok(IntentSnapshot {
                id: row.get(0)?, task_id: row.get(1)?, version: row.get(2)?,
                statement: row.get(3)?, trigger_type: row.get(4)?, reason: row.get(5)?, created_at: row.get(6)?,
            }),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    // ── AgentBranch CRUD ──

    pub fn branch_create(
        &self, task_id: &str, agent_platform: &str, platform_color: &str,
        forked_from_intent_id: &str, source_type: &str,
    ) -> Result<AgentBranch, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO agent_branches (id, task_id, agent_platform, platform_color, forked_from_intent_id, status, progress, output_ref, source_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', NULL, NULL, ?6, ?7, ?7)",
            params![id, task_id, agent_platform, platform_color, forked_from_intent_id, source_type, ts],
        ).map_err(|e| e.to_string())?;
        Ok(AgentBranch { id, task_id: task_id.to_string(), agent_platform: agent_platform.to_string(),
            platform_color: platform_color.to_string(), forked_from_intent_id: forked_from_intent_id.to_string(),
            status: "running".to_string(), progress: None, output_ref: None,
            source_type: source_type.to_string(), created_at: ts.clone(), updated_at: ts })
    }

    pub fn branch_get(&self, id: &str) -> Result<AgentBranch, String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, task_id, agent_platform, platform_color, forked_from_intent_id, status, progress, output_ref, source_type, created_at, updated_at FROM agent_branches WHERE id = ?1",
            params![id],
            |row| Ok(AgentBranch {
                id: row.get(0)?, task_id: row.get(1)?, agent_platform: row.get(2)?,
                platform_color: row.get(3)?, forked_from_intent_id: row.get(4)?,
                status: row.get(5)?, progress: row.get(6)?, output_ref: row.get(7)?,
                source_type: row.get(8)?, created_at: row.get(9)?, updated_at: row.get(10)?,
            }),
        ).map_err(|e| format!("BRANCH_NOT_FOUND: {e}"))
    }

    pub fn branch_list(&self, task_id: &str) -> Result<Vec<AgentBranch>, String> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, agent_platform, platform_color, forked_from_intent_id, status, progress, output_ref, source_type, created_at, updated_at FROM agent_branches WHERE task_id = ?1 ORDER BY created_at ASC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![task_id], |row| Ok(AgentBranch {
            id: row.get(0)?, task_id: row.get(1)?, agent_platform: row.get(2)?,
            platform_color: row.get(3)?, forked_from_intent_id: row.get(4)?,
            status: row.get(5)?, progress: row.get(6)?, output_ref: row.get(7)?,
            source_type: row.get(8)?, created_at: row.get(9)?, updated_at: row.get(10)?,
        })).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }

    pub fn branch_update(&self, id: &str, status: Option<&str>, progress: Option<f64>, output_ref: Option<&str>) -> Result<AgentBranch, String> {
        {
            let conn = self.conn();
            let ts = now();
            if let Some(s) = status {
                conn.execute("UPDATE agent_branches SET status = ?1, updated_at = ?2 WHERE id = ?3", params![s, ts, id])
                    .map_err(|e| e.to_string())?;
            }
            if let Some(p) = progress {
                conn.execute("UPDATE agent_branches SET progress = ?1, updated_at = ?2 WHERE id = ?3", params![p, ts, id])
                    .map_err(|e| e.to_string())?;
            }
            if let Some(o) = output_ref {
                conn.execute("UPDATE agent_branches SET output_ref = ?1, updated_at = ?2 WHERE id = ?3", params![o, ts, id])
                    .map_err(|e| e.to_string())?;
            }
        }
        self.branch_get(id)
    }

    // ── DriftMarker CRUD ──

    pub fn drift_create(&self, branch_id: &str, summary: &str) -> Result<DriftMarker, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO drift_markers (id, branch_id, summary, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, branch_id, summary, ts],
        ).map_err(|e| e.to_string())?;
        Ok(DriftMarker { id, branch_id: branch_id.to_string(), summary: summary.to_string(), created_at: ts })
    }

    pub fn drift_list_for_branch(&self, branch_id: &str) -> Result<Vec<DriftMarker>, String> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, branch_id, summary, created_at FROM drift_markers WHERE branch_id = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![branch_id], |row| Ok(DriftMarker {
            id: row.get(0)?, branch_id: row.get(1)?, summary: row.get(2)?, created_at: row.get(3)?,
        })).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }

    pub fn task_has_drift(&self, task_id: &str) -> Result<bool, String> {
        let conn = self.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM drift_markers d JOIN agent_branches b ON d.branch_id = b.id WHERE b.task_id = ?1",
            params![task_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        Ok(count > 0)
    }

    // ── ResumeNote CRUD ──

    pub fn resume_note_upsert(&self, task_id: &str, content: &str, source: &str) -> Result<ResumeNote, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO resume_notes (id, task_id, content, source, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(task_id) DO UPDATE SET content = ?3, source = ?4, created_at = ?5",
            params![id, task_id, content, source, ts],
        ).map_err(|e| e.to_string())?;
        let note = conn.query_row(
            "SELECT id, task_id, content, source, created_at FROM resume_notes WHERE task_id = ?1",
            params![task_id],
            |row| Ok(ResumeNote { id: row.get(0)?, task_id: row.get(1)?, content: row.get(2)?, source: row.get(3)?, created_at: row.get(4)? }),
        ).map_err(|e| e.to_string())?;
        Ok(note)
    }

    pub fn resume_note_get(&self, task_id: &str) -> Result<Option<ResumeNote>, String> {
        let conn = self.conn();
        let result = conn.query_row(
            "SELECT id, task_id, content, source, created_at FROM resume_notes WHERE task_id = ?1",
            params![task_id],
            |row| Ok(ResumeNote { id: row.get(0)?, task_id: row.get(1)?, content: row.get(2)?, source: row.get(3)?, created_at: row.get(4)? }),
        );
        match result {
            Ok(n) => Ok(Some(n)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    // ── EnvironmentSnapshot CRUD ──

    #[allow(clippy::too_many_arguments)]
    pub fn snapshot_create(&self, task_id: &str, git_branch: Option<&str>, git_status: Option<&str>,
        git_diff_summary: Option<&str>, active_files: Option<&str>, terminal_last_output: Option<&str>,
        window_focus: Option<&str>, agent_states: Option<&str>, completeness: &str,
    ) -> Result<EnvironmentSnapshot, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO environment_snapshots (id, task_id, git_branch, git_status, git_diff_summary, active_files, terminal_last_output, window_focus, agent_states, captured_at, completeness)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![id, task_id, git_branch, git_status, git_diff_summary, active_files, terminal_last_output, window_focus, agent_states, ts, completeness],
        ).map_err(|e| e.to_string())?;
        Ok(EnvironmentSnapshot { id, task_id: task_id.to_string(), git_branch: git_branch.map(String::from),
            git_status: git_status.map(String::from), git_diff_summary: git_diff_summary.map(String::from),
            active_files: active_files.map(String::from), terminal_last_output: terminal_last_output.map(String::from),
            window_focus: window_focus.map(String::from), agent_states: agent_states.map(String::from),
            captured_at: ts, completeness: completeness.to_string() })
    }

    pub fn snapshot_latest(&self, task_id: &str) -> Result<Option<EnvironmentSnapshot>, String> {
        let conn = self.conn();
        let result = conn.query_row(
            "SELECT id, task_id, git_branch, git_status, git_diff_summary, active_files, terminal_last_output, window_focus, agent_states, captured_at, completeness
             FROM environment_snapshots WHERE task_id = ?1 ORDER BY captured_at DESC LIMIT 1",
            params![task_id],
            |row| Ok(EnvironmentSnapshot {
                id: row.get(0)?, task_id: row.get(1)?, git_branch: row.get(2)?,
                git_status: row.get(3)?, git_diff_summary: row.get(4)?, active_files: row.get(5)?,
                terminal_last_output: row.get(6)?, window_focus: row.get(7)?, agent_states: row.get(8)?,
                captured_at: row.get(9)?, completeness: row.get(10)?,
            }),
        );
        match result {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    // ── AgentEvent CRUD ──

    pub fn event_create(&self, branch_id: &str, event_type: &str, summary: Option<&str>, metadata: Option<&str>) -> Result<AgentEvent, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO agent_events (id, branch_id, event_type, summary, metadata, created_at, briefing_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
            params![id, branch_id, event_type, summary, metadata, ts],
        ).map_err(|e| e.to_string())?;
        Ok(AgentEvent { id, branch_id: branch_id.to_string(), event_type: event_type.to_string(),
            summary: summary.map(String::from), metadata: metadata.map(String::from), created_at: ts, briefing_id: None })
    }

    pub fn event_list_unread(&self) -> Result<Vec<AgentEvent>, String> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, branch_id, event_type, summary, metadata, created_at, briefing_id FROM agent_events WHERE briefing_id IS NULL ORDER BY created_at ASC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| Ok(AgentEvent {
            id: row.get(0)?, branch_id: row.get(1)?, event_type: row.get(2)?,
            summary: row.get(3)?, metadata: row.get(4)?, created_at: row.get(5)?, briefing_id: row.get(6)?,
        })).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }

    pub fn event_mark_consumed(&self, event_ids: &[String], briefing_id: &str) -> Result<(), String> {
        let conn = self.conn();
        for eid in event_ids {
            conn.execute("UPDATE agent_events SET briefing_id = ?1 WHERE id = ?2", params![briefing_id, eid])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // ── ReviewLog CRUD ──

    pub fn review_log_create(&self, task_id: &str, branch_id: &str, started_at: &str, duration_seconds: i64, outcome: &str) -> Result<ReviewLog, String> {
        let conn = self.conn();
        let id = new_id();
        conn.execute(
            "INSERT INTO review_logs (id, task_id, branch_id, started_at, duration_seconds, outcome) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, task_id, branch_id, started_at, duration_seconds, outcome],
        ).map_err(|e| e.to_string())?;
        Ok(ReviewLog { id, task_id: task_id.to_string(), branch_id: branch_id.to_string(), started_at: started_at.to_string(), duration_seconds, outcome: outcome.to_string() })
    }

    pub fn review_log_query(&self, task_id: Option<&str>, from_date: Option<&str>, to_date: Option<&str>) -> Result<Vec<ReviewLog>, String> {
        let conn = self.conn();
        let mut sql = String::from("SELECT id, task_id, branch_id, started_at, duration_seconds, outcome FROM review_logs WHERE 1=1");
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        if let Some(tid) = task_id {
            sql.push_str(&format!(" AND task_id = ?{}", param_values.len() + 1));
            param_values.push(Box::new(tid.to_string()));
        }
        if let Some(fd) = from_date {
            sql.push_str(&format!(" AND started_at >= ?{}", param_values.len() + 1));
            param_values.push(Box::new(fd.to_string()));
        }
        if let Some(td) = to_date {
            sql.push_str(&format!(" AND started_at <= ?{}", param_values.len() + 1));
            param_values.push(Box::new(td.to_string()));
        }
        sql.push_str(" ORDER BY started_at DESC");

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params_ref.as_slice(), |row| Ok(ReviewLog {
            id: row.get(0)?, task_id: row.get(1)?, branch_id: row.get(2)?,
            started_at: row.get(3)?, duration_seconds: row.get(4)?, outcome: row.get(5)?,
        })).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }

    // ── Briefing CRUD ──

    pub fn briefing_save(&self, items_json: &str, event_ids: &[String]) -> Result<Briefing, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO briefings (id, generated_at, read_at, items) VALUES (?1, ?2, NULL, ?3)",
            params![id, ts, items_json],
        ).map_err(|e| e.to_string())?;
        for eid in event_ids {
            conn.execute("UPDATE agent_events SET briefing_id = ?1 WHERE id = ?2", params![id, eid])
                .map_err(|e| e.to_string())?;
        }
        Ok(Briefing { id, generated_at: ts, read_at: None, items: items_json.to_string() })
    }

    pub fn briefing_mark_read(&self, briefing_id: &str) -> Result<(), String> {
        let conn = self.conn();
        let ts = now();
        conn.execute("UPDATE briefings SET read_at = ?1 WHERE id = ?2", params![ts, briefing_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Aggregated queries ──

    pub fn get_unreviewed_branch_count(&self) -> Result<i64, String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM agent_branches WHERE status = 'completed' AND id NOT IN (SELECT branch_id FROM review_logs WHERE outcome = 'approved')",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn get_task_card_data(&self, task_id: &str) -> Result<TaskCardData, String> {
        let task = self.task_get(task_id)?;
        let current_intent = self.intent_get_current(task_id)?;
        let branches = self.branch_list(task_id)?;
        let resume_note = self.resume_note_get(task_id)?;
        let latest_snapshot = self.snapshot_latest(task_id)?;
        let has_drift = self.task_has_drift(task_id)?;
        Ok(TaskCardData { task, current_intent, branches, resume_note, latest_snapshot, has_drift })
    }

    pub fn get_graph_data(&self, task_id: &str) -> Result<GraphData, String> {
        let intents = self.intent_list(task_id)?;
        let branches = self.branch_list(task_id)?;
        let task = self.task_get(task_id)?;

        let mut branch_edges = Vec::new();
        for b in &branches {
            let drifts = self.drift_list_for_branch(&b.id)?;
            let has_drift = !drifts.is_empty();
            let drift_summary = drifts.first().map(|d| d.summary.clone());
            branch_edges.push(BranchEdgeData {
                branch_id: b.id.clone(), platform: b.agent_platform.clone(),
                color: b.platform_color.clone(), forked_from_intent_id: b.forked_from_intent_id.clone(),
                status: b.status.clone(), has_drift, drift_summary,
            });
        }

        Ok(GraphData { intent_nodes: intents, branch_edges, current_intent_id: task.current_intent_id })
    }

    pub fn get_task_summary(&self, task_id: &str) -> Result<TaskSummary, String> {
        let task = self.task_get(task_id)?;
        let current_intent = self.intent_get_current(task_id)?;
        let branches = self.branch_list(task_id)?;
        let has_drift = self.task_has_drift(task_id)?;

        let agent_count = branches.len() as i64;
        let running_count = branches.iter().filter(|b| b.status == "running").count() as i64;
        let completed_ids: Vec<&str> = branches.iter().filter(|b| b.status == "completed").map(|b| b.id.as_str()).collect();
        let mut completed_unreviewed = 0i64;
        let conn = self.conn();
        for cid in &completed_ids {
            let approved: i64 = conn.query_row(
                "SELECT COUNT(*) FROM review_logs WHERE branch_id = ?1 AND outcome = 'approved'",
                params![cid], |row| row.get(0),
            ).unwrap_or(0);
            if approved == 0 { completed_unreviewed += 1; }
        }
        let platform_colors: Vec<String> = branches.iter().map(|b| b.platform_color.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect();

        Ok(TaskSummary {
            id: task.id, name: task.name, status: task.status,
            current_intent_statement: current_intent.map(|i| i.statement),
            agent_count, running_count, completed_unreviewed_count: completed_unreviewed,
            has_drift, platform_colors,
        })
    }

    // ── Config queries (used by Platform module) ──

    pub fn get_config(&self) -> Result<AppConfig, String> {
        let conn = self.conn();
        let mut config = default_config();
        let mut stmt = conn
            .prepare("SELECT key, value FROM config")
            .map_err(|err| err.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|err| err.to_string())?;

        for row in rows {
            let (key, value) = row.map_err(|err| err.to_string())?;
            match key.as_str() {
                "llm_mode" => config.llm_mode = value,
                "local_model" => config.local_model = value,
                "cloud_model" => config.cloud_model = Some(value),
                "cloud_endpoint" => config.cloud_endpoint = Some(value),
                _ => {}
            }
        }

        if !matches!(config.llm_mode.as_str(), "local" | "hybrid" | "cloud") {
            return Err("INVALID_LLM_MODE".to_string());
        }

        Ok(config)
    }

    pub fn set_config(&self, config: &AppConfig) -> Result<AppConfig, String> {
        if !matches!(config.llm_mode.as_str(), "local" | "hybrid" | "cloud") {
            return Err("INVALID_LLM_MODE".to_string());
        }

        let conn = self.conn();
        let entries = [
            ("llm_mode", Some(config.llm_mode.as_str())),
            ("local_model", Some(config.local_model.as_str())),
            ("cloud_model", config.cloud_model.as_deref()),
            ("cloud_endpoint", config.cloud_endpoint.as_deref()),
        ];

        for (key, value) in entries {
            if let Some(value) = value {
                conn.execute(
                    "INSERT INTO config (key, value)
                     VALUES (?1, ?2)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    params![key, value],
                )
                .map_err(|err| err.to_string())?;
            } else {
                conn.execute("DELETE FROM config WHERE key = ?1", params![key])
                    .map_err(|err| err.to_string())?;
            }
        }

        drop(conn);
        self.get_config()
    }

    /// Key-value config getter: returns None if the key does not exist.
    pub fn get_config_value(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn();
        let result = conn.query_row(
            "SELECT value FROM config WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Key-value config setter: upserts a single key/value pair.
    pub fn set_config_value(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO config (key, value)
             VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── v2 Brain module queries ──

    pub fn upsert_context(&self, project_key: &str, project_dir: &str, display_name: &str) -> Result<ContextRecord, String> {
        let conn = self.conn();
        let ts = now();
        // Try to find existing
        let existing = conn.query_row(
            "SELECT id, project_key, project_dir, name, manual_assignment_required, status, status_override_until, created_at, updated_at FROM contexts WHERE project_key = ?1",
            params![project_key],
            |row| Ok(ContextRecord {
                id: row.get(0)?,
                project_key: row.get(1)?,
                project_dir: row.get(2)?,
                name: row.get(3)?,
                manual_assignment_required: row.get::<_, i32>(4)? != 0,
                status: row.get(5)?,
                status_override_until: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }),
        );
        match existing {
            Ok(ctx) => {
                conn.execute(
                    "UPDATE contexts SET updated_at = ?1 WHERE id = ?2",
                    params![ts, ctx.id],
                ).map_err(|e| e.to_string())?;
                Ok(ContextRecord { updated_at: ts, ..ctx })
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let id = new_id();
                conn.execute(
                    "INSERT INTO contexts (id, project_key, project_dir, name, manual_assignment_required, status, status_override_until, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 0, 'running', NULL, ?5, ?5)",
                    params![id, project_key, project_dir, display_name, ts],
                ).map_err(|e| e.to_string())?;
                Ok(ContextRecord {
                    id, project_key: project_key.to_string(), project_dir: project_dir.to_string(),
                    name: display_name.to_string(), manual_assignment_required: false,
                    status: "running".to_string(), status_override_until: None,
                    created_at: ts.clone(), updated_at: ts,
                })
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn get_context_by_id(&self, context_id: &str) -> Result<ContextRecord, String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, project_key, project_dir, name, manual_assignment_required, status, status_override_until, created_at, updated_at FROM contexts WHERE id = ?1",
            params![context_id],
            |row| Ok(ContextRecord {
                id: row.get(0)?,
                project_key: row.get(1)?,
                project_dir: row.get(2)?,
                name: row.get(3)?,
                manual_assignment_required: row.get::<_, i32>(4)? != 0,
                status: row.get(5)?,
                status_override_until: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }),
        ).map_err(|e| format!("CONTEXT_NOT_FOUND: {e}"))
    }

    pub fn get_context_by_project_dir(&self, project_dir: &str) -> Result<Option<ContextRecord>, String> {
        let conn = self.conn();
        let result = conn.query_row(
            "SELECT id, project_key, project_dir, name, manual_assignment_required, status, status_override_until, created_at, updated_at FROM contexts WHERE project_dir = ?1",
            params![project_dir],
            |row| Ok(ContextRecord {
                id: row.get(0)?,
                project_key: row.get(1)?,
                project_dir: row.get(2)?,
                name: row.get(3)?,
                manual_assignment_required: row.get::<_, i32>(4)? != 0,
                status: row.get(5)?,
                status_override_until: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }),
        );
        match result {
            Ok(ctx) => Ok(Some(ctx)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn count_active_contexts(&self) -> Result<i64, String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM contexts WHERE status != 'parked'",
            [],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn list_active_contexts(&self) -> Result<Vec<ContextRecord>, String> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_key, project_dir, name, manual_assignment_required, status, status_override_until, created_at, updated_at FROM contexts WHERE status != 'parked' ORDER BY updated_at DESC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| Ok(ContextRecord {
            id: row.get(0)?,
            project_key: row.get(1)?,
            project_dir: row.get(2)?,
            name: row.get(3)?,
            manual_assignment_required: row.get::<_, i32>(4)? != 0,
            status: row.get(5)?,
            status_override_until: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }

    pub fn update_context_status(&self, context_id: &str, status: &str, override_until: Option<&str>) -> Result<ContextRecord, String> {
        {
            let conn = self.conn();
            let ts = now();
            conn.execute(
                "UPDATE contexts SET status = ?1, status_override_until = ?2, updated_at = ?3 WHERE id = ?4",
                params![status, override_until, ts, context_id],
            ).map_err(|e| e.to_string())?;
        }
        self.get_context_by_id(context_id)
    }

    pub fn insert_raw_prompt(&self, context_id: &str, session_path: &str, message_id: &str, role: &str, content: &str) -> Result<RawPromptRecord, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO raw_prompts (id, context_id, session_path, message_id, role, content, captured_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, context_id, session_path, message_id, role, content, ts],
        ).map_err(|e| e.to_string())?;
        Ok(RawPromptRecord {
            id, context_id: context_id.to_string(), session_path: session_path.to_string(),
            message_id: message_id.to_string(), role: role.to_string(),
            content: content.to_string(), captured_at: ts,
        })
    }

    pub fn get_pending_prompts(&self, context_id: &str, limit: i64) -> Result<Vec<RawPromptRecord>, String> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT r.id, r.context_id, r.session_path, r.message_id, r.role, r.content, r.captured_at
             FROM raw_prompts r
             LEFT JOIN prompt_consumptions pc ON r.id = pc.prompt_id
             WHERE r.context_id = ?1 AND pc.prompt_id IS NULL
             ORDER BY r.captured_at ASC
             LIMIT ?2"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![context_id, limit], |row| Ok(RawPromptRecord {
            id: row.get(0)?, context_id: row.get(1)?, session_path: row.get(2)?,
            message_id: row.get(3)?, role: row.get(4)?, content: row.get(5)?,
            captured_at: row.get(6)?,
        })).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }

    /// Cursor-based pagination: returns `(prompts, total_count)`.
    /// If `since_intent_id` is Some, only prompts created after that intent's `created_at`.
    /// If None, returns all pending prompts for the context.
    pub fn get_pending_prompts_cursor(&self, context_id: &str, since_intent_id: Option<&str>) -> Result<(Vec<RawPromptRecord>, i64), String> {
        let conn = self.conn();

        let since_time: Option<String> = if let Some(intent_id) = since_intent_id {
            let t: Result<String, rusqlite::Error> = conn.query_row(
                "SELECT created_at FROM intents_v2 WHERE id = ?1",
                params![intent_id],
                |row| row.get(0),
            );
            match t {
                Ok(ts) => Some(ts),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(format!("INTENT_NOT_FOUND: {intent_id}"));
                }
                Err(e) => return Err(e.to_string()),
            }
        } else {
            None
        };

        let mut result = Vec::new();
        if let Some(ref after_time) = since_time {
            let mut stmt = conn.prepare(
                "SELECT r.id, r.context_id, r.session_path, r.message_id, r.role, r.content, r.captured_at
                 FROM raw_prompts r
                 LEFT JOIN prompt_consumptions pc ON r.id = pc.prompt_id
                 WHERE r.context_id = ?1 AND pc.prompt_id IS NULL AND r.captured_at > ?2
                 ORDER BY r.captured_at ASC"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![context_id, after_time], |row| Ok(RawPromptRecord {
                id: row.get(0)?, context_id: row.get(1)?, session_path: row.get(2)?,
                message_id: row.get(3)?, role: row.get(4)?, content: row.get(5)?,
                captured_at: row.get(6)?,
            })).map_err(|e| e.to_string())?;
            for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        } else {
            let mut stmt = conn.prepare(
                "SELECT r.id, r.context_id, r.session_path, r.message_id, r.role, r.content, r.captured_at
                 FROM raw_prompts r
                 LEFT JOIN prompt_consumptions pc ON r.id = pc.prompt_id
                 WHERE r.context_id = ?1 AND pc.prompt_id IS NULL
                 ORDER BY r.captured_at ASC"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![context_id], |row| Ok(RawPromptRecord {
                id: row.get(0)?, context_id: row.get(1)?, session_path: row.get(2)?,
                message_id: row.get(3)?, role: row.get(4)?, content: row.get(5)?,
                captured_at: row.get(6)?,
            })).map_err(|e| e.to_string())?;
            for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        }

        let count = result.len() as i64;
        Ok((result, count))
    }

    pub fn mark_consumed(&self, prompt_ids: &[String]) -> Result<(), String> {
        let conn = self.conn();
        let ts = now();
        for id in prompt_ids {
            conn.execute(
                "INSERT OR IGNORE INTO prompt_consumptions (prompt_id, processed_at) VALUES (?1, ?2)",
                params![id, ts],
            ).map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    pub fn insert_intent_v2(&self, context_id: &str, tier: &str, content: &str, source: &str, compressed_from: Option<&str>) -> Result<IntentRecord, String> {
        let conn = self.conn();
        let id = new_id();
        let ts = now();
        conn.execute(
            "INSERT INTO intents_v2 (id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, NULL, ?7)",
            params![id, context_id, tier, content, source, ts, compressed_from],
        ).map_err(|e| e.to_string())?;
        Ok(IntentRecord {
            id, context_id: context_id.to_string(), tier: tier.to_string(),
            content: content.to_string(), source: source.to_string(), created_at: ts,
            archived: false, archived_at: None, compressed_from: compressed_from.map(String::from),
        })
    }

    pub fn get_intent_v2(&self, intent_id: &str) -> Result<IntentRecord, String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from FROM intents_v2 WHERE id = ?1",
            params![intent_id],
            |row| Ok(IntentRecord {
                id: row.get(0)?, context_id: row.get(1)?, tier: row.get(2)?,
                content: row.get(3)?, source: row.get(4)?, created_at: row.get(5)?,
                archived: row.get::<_, i32>(6)? != 0, archived_at: row.get(7)?,
                compressed_from: row.get(8)?,
            }),
        ).map_err(|e| format!("INTENT_NOT_FOUND: {e}"))
    }

    pub fn get_stale_intents(&self, tier: &str, older_than: i64) -> Result<Vec<IntentRecord>, String> {
        let conn = self.conn();
        // `older_than` is a unix epoch timestamp; select intents of the given tier
        // whose created_at is before that threshold.
        let mut stmt = conn.prepare(
            "SELECT id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from
             FROM intents_v2
             WHERE archived = 0
             AND tier = ?1
             AND strftime('%s', created_at) <= CAST(?2 AS TEXT)
             ORDER BY created_at ASC"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![tier, older_than], |row| Ok(IntentRecord {
            id: row.get(0)?, context_id: row.get(1)?, tier: row.get(2)?,
            content: row.get(3)?, source: row.get(4)?, created_at: row.get(5)?,
            archived: row.get::<_, i32>(6)? != 0, archived_at: row.get(7)?,
            compressed_from: row.get(8)?,
        })).map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for r in rows { result.push(r.map_err(|e| e.to_string())?); }
        Ok(result)
    }

    pub fn archive_intents(&self, intent_ids: &[String]) -> Result<i32, String> {
        let conn = self.conn();
        let ts = now();
        let mut updated: i32 = 0;
        for id in intent_ids {
            let rows = conn.execute(
                "UPDATE intents_v2 SET archived = 1, archived_at = ?1 WHERE id = ?2",
                params![ts, id],
            ).map_err(|e| e.to_string())?;
            updated += rows as i32;
        }
        Ok(updated)
    }

    pub fn get_intents_for_context(&self, context_id: &str, limit: i64, before_id: Option<&str>) -> Result<Vec<IntentRecord>, String> {
        let conn = self.conn();
        if let Some(bid) = before_id {
            let before_time: String = conn.query_row(
                "SELECT created_at FROM intents_v2 WHERE id = ?1",
                params![bid],
                |row| row.get(0),
            ).map_err(|e| format!("INTENT_NOT_FOUND for cursor: {e}"))?;
            let mut stmt = conn.prepare(
                "SELECT id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from
                 FROM intents_v2
                 WHERE context_id = ?1 AND archived = 0 AND created_at < ?2
                 ORDER BY created_at DESC
                 LIMIT ?3"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![context_id, before_time, limit], |row| Ok(IntentRecord {
                id: row.get(0)?, context_id: row.get(1)?, tier: row.get(2)?,
                content: row.get(3)?, source: row.get(4)?, created_at: row.get(5)?,
                archived: row.get::<_, i32>(6)? != 0, archived_at: row.get(7)?,
                compressed_from: row.get(8)?,
            })).map_err(|e| e.to_string())?;
            let mut result = Vec::new();
            for r in rows { result.push(r.map_err(|e| e.to_string())?); }
            Ok(result)
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from
                 FROM intents_v2
                 WHERE context_id = ?1 AND archived = 0
                 ORDER BY created_at DESC
                 LIMIT ?2"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![context_id, limit], |row| Ok(IntentRecord {
                id: row.get(0)?, context_id: row.get(1)?, tier: row.get(2)?,
                content: row.get(3)?, source: row.get(4)?, created_at: row.get(5)?,
                archived: row.get::<_, i32>(6)? != 0, archived_at: row.get(7)?,
                compressed_from: row.get(8)?,
            })).map_err(|e| e.to_string())?;
            let mut result = Vec::new();
            for r in rows { result.push(r.map_err(|e| e.to_string())?); }
            Ok(result)
        }
    }

    pub fn count_archived_intents(&self, context_id: &str) -> Result<i64, String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM intents_v2 WHERE context_id = ?1 AND archived = 1",
            params![context_id],
            |row| row.get(0),
        ).map_err(|e| e.to_string())
    }

    pub fn get_intents_compressed_from(&self, compressed_intent_id: &str) -> Result<Vec<IntentRecord>, String> {
        // Single lock acquisition to avoid deadlock
        let conn = self.conn();

        // First get the compressed intent to find source IDs
        let intent = conn.query_row(
            "SELECT id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from FROM intents_v2 WHERE id = ?1",
            params![compressed_intent_id],
            |row| Ok(IntentRecord {
                id: row.get(0)?, context_id: row.get(1)?, tier: row.get(2)?,
                content: row.get(3)?, source: row.get(4)?, created_at: row.get(5)?,
                archived: row.get::<_, i32>(6)? != 0, archived_at: row.get(7)?,
                compressed_from: row.get(8)?,
            }),
        ).map_err(|e| format!("INTENT_NOT_FOUND: {e}"))?;

        let source_ids_json = intent.compressed_from.ok_or("NOT_COMPRESSED: intent has no compressed_from field")?;
        let source_ids: Vec<String> = serde_json::from_str(&source_ids_json)
            .map_err(|e| format!("Failed to parse compressed_from: {e}"))?;

        let mut result = Vec::new();
        for sid in &source_ids {
            if let Ok(r) = conn.query_row(
                "SELECT id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from FROM intents_v2 WHERE id = ?1",
                params![sid],
                |row| Ok(IntentRecord {
                    id: row.get(0)?, context_id: row.get(1)?, tier: row.get(2)?,
                    content: row.get(3)?, source: row.get(4)?, created_at: row.get(5)?,
                    archived: row.get::<_, i32>(6)? != 0, archived_at: row.get(7)?,
                    compressed_from: row.get(8)?,
                }),
            ) { result.push(r) }
        }
        Ok(result)
    }

    pub fn get_latest_intent_for_context(&self, context_id: &str) -> Result<Option<IntentRecord>, String> {
        let conn = self.conn();
        let result = conn.query_row(
            "SELECT id, context_id, tier, content, source, created_at, archived, archived_at, compressed_from
             FROM intents_v2
             WHERE context_id = ?1 AND archived = 0
             ORDER BY created_at DESC LIMIT 1",
            params![context_id],
            |row| Ok(IntentRecord {
                id: row.get(0)?, context_id: row.get(1)?, tier: row.get(2)?,
                content: row.get(3)?, source: row.get(4)?, created_at: row.get(5)?,
                archived: row.get::<_, i32>(6)? != 0, archived_at: row.get(7)?,
                compressed_from: row.get(8)?,
            }),
        );
        match result {
            Ok(r) => Ok(Some(r)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_crud() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("test task").unwrap();
        assert_eq!(task.name, "test task");
        assert_eq!(task.status, "active");

        let fetched = db.task_get(&task.id).unwrap();
        assert_eq!(fetched.id, task.id);

        let all = db.task_list(None).unwrap();
        assert_eq!(all.len(), 1);

        let parked = db.task_update_status(&task.id, "parked").unwrap();
        assert_eq!(parked.status, "parked");
        assert!(parked.parked_at.is_some());
    }

    #[test]
    fn test_intent_chain() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("intent test").unwrap();

        let v1 = db.intent_create(&task.id, "refactor auth module", "initial", None).unwrap();
        assert_eq!(v1.version, 1);

        let v2 = db.intent_create(&task.id, "only refactor JWT", "refinement", Some("scope too broad")).unwrap();
        assert_eq!(v2.version, 2);

        let chain = db.intent_list(&task.id).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].version, 1);
        assert_eq!(chain[1].version, 2);

        let current = db.intent_get_current(&task.id).unwrap().unwrap();
        assert_eq!(current.id, v2.id);
    }

    #[test]
    fn test_branch_and_drift() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("branch test").unwrap();
        let intent = db.intent_create(&task.id, "test intent", "initial", None).unwrap();

        let branch = db.branch_create(&task.id, "claude_code", "#14B8A6", &intent.id, "auto").unwrap();
        assert_eq!(branch.status, "running");

        let updated = db.branch_update(&branch.id, Some("completed"), Some(1.0), None).unwrap();
        assert_eq!(updated.status, "completed");

        assert!(!db.task_has_drift(&task.id).unwrap());

        db.drift_create(&branch.id, "modified session management out of scope").unwrap();
        assert!(db.task_has_drift(&task.id).unwrap());
    }

    #[test]
    fn test_config_crud() {
        let db = Database::in_memory().unwrap();

        // Default config
        let config = db.get_config().unwrap();
        assert_eq!(config.llm_mode, "local");
        assert_eq!(config.local_model, "qwen2.5:7b");

        // Set config
        let new_config = AppConfig {
            llm_mode: "hybrid".to_string(),
            local_model: "llama3".to_string(),
            cloud_model: Some("claude-sonnet".to_string()),
            cloud_endpoint: Some("https://api.anthropic.com".to_string()),
        };
        let saved = db.set_config(&new_config).unwrap();
        assert_eq!(saved.llm_mode, "hybrid");
        assert_eq!(saved.local_model, "llama3");
        assert_eq!(saved.cloud_model, Some("claude-sonnet".to_string()));
    }

    #[test]
    fn test_v2_context_crud() {
        let db = Database::in_memory().unwrap();

        let ctx = db.upsert_context("key1", "/home/user/proj", "My Project").unwrap();
        assert_eq!(ctx.project_key, "key1");
        assert_eq!(ctx.project_dir, "/home/user/proj");
        assert_eq!(ctx.status, "running");

        // Upsert same context returns same id
        let ctx2 = db.upsert_context("key1", "/home/user/proj", "My Project").unwrap();
        assert_eq!(ctx.id, ctx2.id);

        let fetched = db.get_context_by_id(&ctx.id).unwrap();
        assert_eq!(fetched.id, ctx.id);

        let by_dir = db.get_context_by_project_dir("/home/user/proj").unwrap();
        assert!(by_dir.is_some());

        let count = db.count_active_contexts().unwrap();
        assert_eq!(count, 1);

        let active = db.list_active_contexts().unwrap();
        assert_eq!(active.len(), 1);
    }

    #[test]
    fn test_v2_raw_prompts() {
        let db = Database::in_memory().unwrap();
        let ctx = db.upsert_context("key1", "/home/user/proj", "My Project").unwrap();

        let prompt = db.insert_raw_prompt(&ctx.id, "/session/1", "msg-1", "user", "Fix the bug").unwrap();
        assert_eq!(prompt.content, "Fix the bug");

        let pending = db.get_pending_prompts(&ctx.id, 10).unwrap();
        assert_eq!(pending.len(), 1);

        db.mark_consumed(&[prompt.id]).unwrap();

        let pending2 = db.get_pending_prompts(&ctx.id, 10).unwrap();
        assert_eq!(pending2.len(), 0);
    }

    #[test]
    fn test_v2_intents() {
        let db = Database::in_memory().unwrap();
        let ctx = db.upsert_context("key1", "/home/user/proj", "My Project").unwrap();

        let intent = db.insert_intent_v2(&ctx.id, "narrative", "Working on auth", "auto", None).unwrap();
        assert_eq!(intent.tier, "narrative");

        let fetched = db.get_intent_v2(&intent.id).unwrap();
        assert_eq!(fetched.content, "Working on auth");

        let latest = db.get_latest_intent_for_context(&ctx.id).unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().id, intent.id);
    }
}
