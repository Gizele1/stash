use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::watcher::JsonlMessage;

/// Tracks byte offsets for each watched JSONL file so we only read new lines.
#[derive(Default)]
pub struct FileTracker {
    offsets: HashMap<PathBuf, u64>,
}

impl FileTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read only new lines from a JSONL file since last read.
    /// Returns parsed JsonlMessage entries (only user messages).
    pub fn read_new_lines(&mut self, path: &Path) -> Vec<JsonlMessage> {
        let mut messages = Vec::new();

        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => {
                // File gone — remove offset so we start fresh if it reappears
                self.offsets.remove(path);
                return messages;
            }
        };

        let metadata = match file.metadata() {
            Ok(m) => m,
            Err(_) => return messages,
        };

        let file_len = metadata.len();
        let prev_offset = self.offsets.get(path).copied().unwrap_or(0);

        // File was truncated or rotated — reset offset
        if file_len < prev_offset {
            self.offsets.insert(path.to_path_buf(), 0);
            return self.read_new_lines(path);
        }

        // No new data
        if file_len == prev_offset {
            return messages;
        }

        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(prev_offset)).is_err() {
            return messages;
        }

        let mut current_offset = prev_offset;
        let mut line_buf = String::new();

        loop {
            line_buf.clear();
            match reader.read_line(&mut line_buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    current_offset += n as u64;
                    if let Some(msg) = parse_jsonl_line(&line_buf) {
                        messages.push(msg);
                    }
                }
                Err(_) => break,
            }
        }

        self.offsets.insert(path.to_path_buf(), current_offset);
        messages
    }

    /// Handle file rotation: if a file was deleted, remove its offset.
    pub fn handle_file_removed(&mut self, path: &Path) {
        self.offsets.remove(path);
    }

    /// Get current offset for a file (for testing).
    #[cfg(test)]
    pub fn get_offset(&self, path: &Path) -> Option<u64> {
        self.offsets.get(path).copied()
    }
}

/// Parse a single JSONL line into a JsonlMessage if it's a user message.
/// Returns None for non-user messages, invalid JSON, or empty lines.
pub fn parse_jsonl_line(line: &str) -> Option<JsonlMessage> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let obj: serde_json::Value = serde_json::from_str(trimmed).ok()?;

    // Must be a "user" type message
    let msg_type = obj.get("type").and_then(|v| v.as_str())?;
    if msg_type != "user" {
        return None;
    }

    // Extract content from message.content
    let content = obj
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();

    if content.is_empty() {
        return None;
    }

    let timestamp = obj
        .get("timestamp")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let session_id = obj
        .get("sessionId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let cwd = obj
        .get("cwd")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Derive project_dir and display_name from cwd
    let project_dir = cwd.clone();
    let display_name = Path::new(&cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // Derive project_hash from the file path's parent directory name
    // (Claude Code uses hashed project paths as directory names)
    let project_hash = String::new();

    Some(JsonlMessage {
        role: "user".to_string(),
        content,
        timestamp,
        session_id,
        project_hash,
        project_dir,
        display_name,
    })
}
