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
    fn test_resume_note_upsert() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("note test").unwrap();

        let n1 = db.resume_note_upsert(&task.id, "auto note", "auto").unwrap();
        assert_eq!(n1.content, "auto note");

        let n2 = db.resume_note_upsert(&task.id, "manual override", "manual").unwrap();
        assert_eq!(n2.content, "manual override");
        assert_eq!(n2.task_id, task.id);
    }

    #[test]
    fn test_environment_snapshot() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("snapshot test").unwrap();

        let snap = db.snapshot_create(&task.id, Some("main"), None, None, None, None, None, None, "full").unwrap();
        assert_eq!(snap.completeness, "full");

        let latest = db.snapshot_latest(&task.id).unwrap().unwrap();
        assert_eq!(latest.id, snap.id);
    }

    #[test]
    fn test_events_and_briefing() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("event test").unwrap();
        let intent = db.intent_create(&task.id, "test", "initial", None).unwrap();
        let branch = db.branch_create(&task.id, "claude_code", "#14B8A6", &intent.id, "auto").unwrap();

        db.event_create(&branch.id, "completed", Some("done"), None).unwrap();
        db.event_create(&branch.id, "commit_detected", Some("abc123"), None).unwrap();

        let unread = db.event_list_unread().unwrap();
        assert_eq!(unread.len(), 2);

        let event_ids: Vec<String> = unread.iter().map(|e| e.id.clone()).collect();
        let briefing = db.briefing_save("[]", &event_ids).unwrap();
        assert!(briefing.read_at.is_none());

        let unread_after = db.event_list_unread().unwrap();
        assert_eq!(unread_after.len(), 0);
    }

    #[test]
    fn test_review_log() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("review test").unwrap();
        let intent = db.intent_create(&task.id, "test", "initial", None).unwrap();
        let branch = db.branch_create(&task.id, "claude_code", "#14B8A6", &intent.id, "auto").unwrap();

        db.review_log_create(&task.id, &branch.id, "2026-01-01T00:00:00Z", 45, "approved").unwrap();
        let logs = db.review_log_query(Some(&task.id), None, None).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].duration_seconds, 45);
    }

    #[test]
    fn test_unreviewed_branch_count() {
        let db = Database::in_memory().unwrap();
        let task = db.task_create("count test").unwrap();
        let intent = db.intent_create(&task.id, "test", "initial", None).unwrap();

        let b1 = db.branch_create(&task.id, "claude_code", "#14B8A6", &intent.id, "auto").unwrap();
        db.branch_update(&b1.id, Some("completed"), None, None).unwrap();
        let b2 = db.branch_create(&task.id, "codex", "#F59E0B", &intent.id, "manual").unwrap();
        db.branch_update(&b2.id, Some("completed"), None, None).unwrap();

        assert_eq!(db.get_unreviewed_branch_count().unwrap(), 2);

        db.review_log_create(&task.id, &b1.id, "2026-01-01T00:00:00Z", 30, "approved").unwrap();
        assert_eq!(db.get_unreviewed_branch_count().unwrap(), 1);
    }
}
