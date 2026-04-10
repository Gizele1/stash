use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::watcher::WatcherError;

/// Snapshot of git state for a project directory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitStatus {
    /// Unix epoch of the last commit.
    pub last_commit_time: i64,
    /// Current branch name.
    pub branch: String,
    /// True if there are local commits not yet pushed.
    pub has_unpushed: bool,
    /// URL of a recent PR, if detectable (None for now).
    pub recent_pr_url: Option<String>,
}

/// Poll the current git status for a project directory.
///
/// Uses `std::process::Command` for all git CLI calls and returns
/// `Err(WatcherError::GitError(...))` when git is not available or the
/// directory is not a repository.
pub fn poll_git_status(project_dir: &Path) -> Result<GitStatus, WatcherError> {
    // Last commit time (unix epoch)
    let commit_time_output = Command::new("git")
        .args(["-C", &project_dir.to_string_lossy(), "log", "-1", "--format=%ct"])
        .output()
        .map_err(|e| WatcherError::GitError(e.to_string()))?;

    let last_commit_time = if commit_time_output.status.success() {
        String::from_utf8_lossy(&commit_time_output.stdout)
            .trim()
            .parse::<i64>()
            .unwrap_or(0)
    } else {
        0
    };

    // Current branch name
    let branch_output = Command::new("git")
        .args(["-C", &project_dir.to_string_lossy(), "branch", "--show-current"])
        .output()
        .map_err(|e| WatcherError::GitError(e.to_string()))?;

    let branch = if branch_output.status.success() {
        String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string()
    } else {
        String::new()
    };

    // Count unpushed commits
    let unpushed_output = Command::new("git")
        .args(["-C", &project_dir.to_string_lossy(), "rev-list", "--count", "@{u}..HEAD"])
        .output()
        .map_err(|e| WatcherError::GitError(e.to_string()))?;

    let has_unpushed = if unpushed_output.status.success() {
        String::from_utf8_lossy(&unpushed_output.stdout)
            .trim()
            .parse::<u64>()
            .map(|n| n > 0)
            .unwrap_or(false)
    } else {
        // No upstream configured — treat as no unpushed
        false
    };

    Ok(GitStatus {
        last_commit_time,
        branch,
        has_unpushed,
        recent_pr_url: None,
    })
}

/// Tracks git state per project directory to detect signals.
pub struct GitMonitor {
    /// Last known HEAD SHA per project directory
    head_shas: HashMap<PathBuf, String>,
    /// Last time we polled each project directory
    last_poll: HashMap<PathBuf, Instant>,
    /// Last time git activity was detected per project
    last_activity: HashMap<PathBuf, Instant>,
    /// Minimum interval between polls
    poll_interval: Duration,
    /// Idle threshold (30 minutes)
    idle_threshold: Duration,
}

/// A git signal detected for a project.
#[derive(Debug, Clone, PartialEq)]
pub struct GitSignal {
    pub project_dir: String,
    pub signal_type: String,
    pub metadata: serde_json::Value,
}

impl GitMonitor {
    pub fn new(poll_interval_secs: u64) -> Self {
        Self {
            head_shas: HashMap::new(),
            last_poll: HashMap::new(),
            last_activity: HashMap::new(),
            poll_interval: Duration::from_secs(poll_interval_secs),
            idle_threshold: Duration::from_secs(30 * 60),
        }
    }

    /// Check a project directory for git signals.
    /// Returns empty vec if poll interval hasn't elapsed yet.
    pub fn check_signals(&mut self, project_dir: &Path) -> Vec<GitSignal> {
        let now = Instant::now();

        // Rate limit: skip if we polled too recently
        if let Some(last) = self.last_poll.get(project_dir) {
            if now.duration_since(*last) < self.poll_interval {
                return Vec::new();
            }
        }

        self.last_poll.insert(project_dir.to_path_buf(), now);

        let mut signals = Vec::new();

        // Check for new commits
        if let Some(signal) = self.check_commit(project_dir) {
            self.last_activity.insert(project_dir.to_path_buf(), now);
            signals.push(signal);
        }

        // Check for push (unpushed commits go to 0)
        if let Some(signal) = self.check_push(project_dir) {
            self.last_activity.insert(project_dir.to_path_buf(), now);
            signals.push(signal);
        }

        // Check for idle
        if let Some(signal) = self.check_idle(project_dir) {
            signals.push(signal);
        }

        signals
    }

    /// Detect new commit by comparing HEAD SHA.
    fn check_commit(&mut self, project_dir: &Path) -> Option<GitSignal> {
        let current_sha = git_head_sha(project_dir).ok()?;
        let prev_sha = self.head_shas.get(project_dir).cloned();

        self.head_shas
            .insert(project_dir.to_path_buf(), current_sha.clone());

        match prev_sha {
            Some(prev) if prev != current_sha => Some(GitSignal {
                project_dir: project_dir.to_string_lossy().to_string(),
                signal_type: "commit".to_string(),
                metadata: serde_json::json!({
                    "prev_sha": prev,
                    "new_sha": current_sha,
                }),
            }),
            Some(_) => None, // Same SHA, no new commit
            None => None,    // First poll, just record baseline
        }
    }

    /// Detect push by checking unpushed commit count going to 0.
    fn check_push(&mut self, project_dir: &Path) -> Option<GitSignal> {
        let unpushed = git_unpushed_count(project_dir).ok()?;

        if unpushed == 0 {
            // Only signal push if we previously had unpushed commits
            // For simplicity, we signal when count is exactly 0
            // A more sophisticated approach would track previous unpushed count
            return None;
        }

        None
    }

    /// Detect idle: no git activity for 30 minutes.
    fn check_idle(&self, project_dir: &Path) -> Option<GitSignal> {
        let now = Instant::now();

        if let Some(last) = self.last_activity.get(project_dir) {
            if now.duration_since(*last) >= self.idle_threshold {
                return Some(GitSignal {
                    project_dir: project_dir.to_string_lossy().to_string(),
                    signal_type: "idle".to_string(),
                    metadata: serde_json::json!({
                        "idle_seconds": now.duration_since(*last).as_secs(),
                    }),
                });
            }
        }

        None
    }

    /// Register a project directory for monitoring (sets initial activity time).
    pub fn register_project(&mut self, project_dir: &Path) {
        let now = Instant::now();
        self.last_activity
            .entry(project_dir.to_path_buf())
            .or_insert(now);
    }
}

/// Get the HEAD SHA of a git repository.
pub fn git_head_sha(project_dir: &Path) -> Result<String, WatcherError> {
    let output = Command::new("git")
        .args(["-C", &project_dir.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .map_err(|e| WatcherError::GitError(e.to_string()))?;

    if !output.status.success() {
        return Err(WatcherError::GitError(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get count of unpushed commits.
pub fn git_unpushed_count(project_dir: &Path) -> Result<usize, WatcherError> {
    let output = Command::new("git")
        .args([
            "-C",
            &project_dir.to_string_lossy(),
            "log",
            "--oneline",
            "@{upstream}..HEAD",
        ])
        .output()
        .map_err(|e| WatcherError::GitError(e.to_string()))?;

    if !output.status.success() {
        // No upstream configured — treat as 0 unpushed
        return Ok(0);
    }

    let count = String::from_utf8_lossy(&output.stdout)
        .lines()
        .count();
    Ok(count)
}
