use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Trait for watching agent sessions across different platforms.
pub trait AgentWatcher: Send + Sync {
    fn platform_name(&self) -> &str;
    fn detect_sessions(&self) -> Vec<DetectedSession>;
}

#[derive(Debug, Clone)]
pub struct DetectedSession {
    pub session_id: String,
    pub platform: String,
    pub working_dir: Option<PathBuf>,
    pub status: SessionStatus,
    pub last_output: Option<String>,
    /// User messages extracted from this session (newest first)
    pub user_messages: Vec<UserMessage>,
}

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Running,
    Idle,
    Completed,
    Error,
}

/// Claude Code session watcher — reads from ~/.claude/projects/ session files.
pub struct ClaudeCodeWatcher {
    home_dir: PathBuf,
    /// Track how many lines we've already read per file to only process new lines
    file_offsets: std::sync::Mutex<HashMap<PathBuf, usize>>,
}

impl ClaudeCodeWatcher {
    pub fn new() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            home_dir,
            file_offsets: std::sync::Mutex::new(HashMap::new()),
        }
    }

    #[cfg(test)]
    pub fn with_home(home_dir: PathBuf) -> Self {
        Self {
            home_dir,
            file_offsets: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn projects_dir(&self) -> PathBuf {
        self.home_dir.join(".claude").join("projects")
    }

    /// Scan all project directories for .jsonl session files
    fn scan_session_files(&self) -> Vec<(PathBuf, PathBuf)> {
        let projects_dir = self.projects_dir();
        if !projects_dir.exists() {
            return Vec::new();
        }

        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let project_dir = entry.path();
                if !project_dir.is_dir() {
                    continue;
                }
                if let Ok(project_entries) = fs::read_dir(&project_dir) {
                    for pe in project_entries.flatten() {
                        let path = pe.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                            files.push((project_dir.clone(), path));
                        }
                    }
                }
            }
        }
        files
    }

    /// Parse a session JSONL file, returning only NEW lines since last read
    fn parse_session_file(&self, path: &PathBuf) -> Option<DetectedSession> {
        let file = fs::File::open(path).ok()?;
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().map_while(|l| l.ok()).collect();

        let mut offsets = self.file_offsets.lock().ok()?;
        let prev_offset = offsets.get(path).copied().unwrap_or(0);

        // If no new lines, skip
        if lines.len() <= prev_offset {
            return None;
        }

        let mut session_id = String::new();
        let mut cwd: Option<PathBuf> = None;
        let mut user_messages = Vec::new();
        let mut last_timestamp = String::new();

        // Process ALL lines for metadata, but only NEW lines for messages
        for (i, line) in lines.iter().enumerate() {
            let obj: serde_json::Value = serde_json::from_str(line).ok()?;

            // Extract session metadata from any line
            if session_id.is_empty() {
                if let Some(sid) = obj.get("sessionId").and_then(|v| v.as_str()) {
                    session_id = sid.to_string();
                }
            }
            if cwd.is_none() {
                if let Some(c) = obj.get("cwd").and_then(|v| v.as_str()) {
                    cwd = Some(PathBuf::from(c));
                }
            }

            // Only extract user messages from NEW lines
            if i >= prev_offset {
                if let Some(msg_type) = obj.get("type").and_then(|v| v.as_str()) {
                    if msg_type == "user" {
                        if let Some(msg) = obj.get("message") {
                            if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
                                let content = content.trim().to_string();
                                // Skip system/hook messages and short replies
                                if !content.is_empty()
                                    && content.chars().count() > 5
                                    && !content.starts_with('<')
                                    && !content.starts_with('/')
                                    && !content.contains("<system-reminder>")
                                    && !content.contains("<task-notification>")
                                    && !content.contains("<command-name>")
                                {
                                    let ts = obj
                                        .get("timestamp")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    last_timestamp = ts.clone();
                                    user_messages.push(UserMessage {
                                        content,
                                        timestamp: ts,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Update offset
        offsets.insert(path.clone(), lines.len());

        if session_id.is_empty() {
            return None;
        }

        // Determine status: if last line is recent (within 60s), consider running
        let status = if !last_timestamp.is_empty() {
            SessionStatus::Running
        } else {
            SessionStatus::Idle
        };

        let last_msg = user_messages.last().map(|m| m.content.clone());

        Some(DetectedSession {
            session_id,
            platform: "claude-code".to_string(),
            working_dir: cwd,
            status,
            last_output: last_msg,
            user_messages,
        })
    }
}

impl Default for ClaudeCodeWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentWatcher for ClaudeCodeWatcher {
    fn platform_name(&self) -> &str {
        "claude-code"
    }

    fn detect_sessions(&self) -> Vec<DetectedSession> {
        let files = self.scan_session_files();
        let mut sessions = Vec::new();

        for (_project_dir, file_path) in files {
            if let Some(session) = self.parse_session_file(&file_path) {
                // Only return sessions that have new user messages
                if !session.user_messages.is_empty() {
                    sessions.push(session);
                }
            }
        }

        sessions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_code_watcher_no_dir() {
        let watcher = ClaudeCodeWatcher::with_home(PathBuf::from("/nonexistent"));
        assert_eq!(watcher.platform_name(), "claude-code");
        assert!(watcher.detect_sessions().is_empty());
    }

    #[test]
    fn test_parse_returns_none_for_missing_file() {
        let watcher = ClaudeCodeWatcher::with_home(PathBuf::from("/tmp"));
        let result = watcher.parse_session_file(&PathBuf::from("/nonexistent/file.jsonl"));
        assert!(result.is_none());
    }
}
