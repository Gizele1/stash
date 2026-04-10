pub mod git_monitor;
pub mod jsonl;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use tracing;

use crate::watcher::git_monitor::GitMonitor;
use crate::watcher::jsonl::FileTracker;

// ── Types ──

/// A parsed JSONL message from a Claude Code session file.
/// This will migrate to `crate::brain::JsonlMessage` once the Brain module is built.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsonlMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub session_id: String,
    pub project_hash: String,
    pub project_dir: String,
    pub display_name: String,
}

// ── Config ──

/// Configuration for the Watcher.
#[derive(Debug)]
pub struct WatcherConfig {
    /// Base directory for Claude Code projects (e.g. ~/.claude/projects)
    pub claude_base_dir: PathBuf,
    /// Debounce interval in milliseconds for file system events
    pub debounce_ms: u64,
    /// Git polling interval in seconds
    pub git_poll_interval_secs: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            claude_base_dir: home.join(".claude").join("projects"),
            debounce_ms: 500,
            git_poll_interval_secs: 30,
        }
    }
}

// ── Errors ──

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("Failed to initialize watcher: {0}")]
    InitFailed(String),
    #[error("Git error: {0}")]
    GitError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// ── Type Aliases ──

/// Callback type for git signal events.
type GitCallback = Box<dyn Fn(&str, &str, &serde_json::Value) + Send + 'static>;

/// Callback type for JSONL append events. First arg is the file path, second is the messages.
type JsonlCallback = Box<dyn Fn(&str, Vec<JsonlMessage>) + Send + 'static>;

/// Callback type for JSONL file rotation events: `(old_path, new_path)`.
type RotationCallback = Box<dyn Fn(&str, &str) + Send + 'static>;

// ── Watcher ──

#[derive(Debug)]
pub struct Watcher {
    config: WatcherConfig,
    stop_flag: Arc<AtomicBool>,
}

impl Watcher {
    pub fn new(config: WatcherConfig) -> Result<Self, WatcherError> {
        // Validate that the base dir path is reasonable (don't require it to exist yet)
        if config.claude_base_dir.as_os_str().is_empty() {
            return Err(WatcherError::InitFailed(
                "claude_base_dir cannot be empty".to_string(),
            ));
        }

        Ok(Self {
            config,
            stop_flag: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Start watching. Calls `on_jsonl` for new JSONL messages (with file path as first arg),
    /// `on_git` for git signals, and `on_rotation` when a watched JSONL file is removed and a
    /// new one appears in the same project directory.
    /// Returns a JoinHandle for the background thread.
    pub fn start(
        &mut self,
        on_jsonl: JsonlCallback,
        on_git: GitCallback,
        on_rotation: RotationCallback,
    ) -> Result<thread::JoinHandle<()>, WatcherError> {
        let stop_flag = self.stop_flag.clone();
        let base_dir = self.config.claude_base_dir.clone();
        let debounce_ms = self.config.debounce_ms;
        let git_poll_secs = self.config.git_poll_interval_secs;

        // Reset stop flag
        self.stop_flag.store(false, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            if let Err(e) = run_watcher_loop(
                stop_flag,
                base_dir,
                debounce_ms,
                git_poll_secs,
                on_jsonl,
                on_git,
                on_rotation,
            ) {
                tracing::error!("Watcher loop failed: {}", e);
            }
        });

        Ok(handle)
    }

    /// Stop watching.
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }
}

/// Main watcher loop that runs in a background thread.
fn run_watcher_loop(
    stop_flag: Arc<AtomicBool>,
    base_dir: PathBuf,
    debounce_ms: u64,
    git_poll_secs: u64,
    on_jsonl: JsonlCallback,
    on_git: GitCallback,
    on_rotation: RotationCallback,
) -> Result<(), WatcherError> {
    let mut file_tracker = FileTracker::new();
    let mut git_monitor = GitMonitor::new(git_poll_secs);

    // Set up notify watcher with channel
    let (tx, rx) = std::sync::mpsc::channel();

    let notify_config = Config::default()
        .with_poll_interval(Duration::from_millis(debounce_ms));

    let mut watcher = RecommendedWatcher::new(tx, notify_config)
        .map_err(|e| WatcherError::InitFailed(e.to_string()))?;

    // Watch the base directory if it exists
    if base_dir.exists() {
        watcher
            .watch(&base_dir, RecursiveMode::Recursive)
            .map_err(|e| WatcherError::InitFailed(e.to_string()))?;
        tracing::info!("Watching directory: {:?}", base_dir);
    } else {
        tracing::warn!(
            "Claude projects directory does not exist yet: {:?}. Will retry.",
            base_dir
        );
    }

    let mut watching = base_dir.exists();

    // Track known project directories for git monitoring
    let mut known_project_dirs: Vec<PathBuf> = Vec::new();
    scan_project_dirs(&base_dir, &mut known_project_dirs);

    // Main event loop
    while !stop_flag.load(Ordering::SeqCst) {
        // If not yet watching, try to start
        if !watching && base_dir.exists() {
            if let Ok(()) = watcher.watch(&base_dir, RecursiveMode::Recursive) {
                watching = true;
                tracing::info!("Started watching: {:?}", base_dir);
            }
        }

        // Process file system events with a timeout so we can check stop_flag
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                handle_fs_event(
                    &event,
                    &mut file_tracker,
                    &on_jsonl,
                    &on_rotation,
                    &mut known_project_dirs,
                );
            }
            Ok(Err(e)) => {
                tracing::warn!("Notify error: {}", e);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout — continue to git polling
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                tracing::error!("Notify channel disconnected");
                break;
            }
        }

        // Git signal polling
        for project_dir in &known_project_dirs {
            let signals = git_monitor.check_signals(project_dir);
            for signal in signals {
                on_git(&signal.project_dir, &signal.signal_type, &signal.metadata);
            }
        }
    }

    tracing::info!("Watcher stopped");
    Ok(())
}

/// Handle a filesystem event from notify.
fn handle_fs_event(
    event: &notify::Event,
    file_tracker: &mut FileTracker,
    on_jsonl: &dyn Fn(&str, Vec<JsonlMessage>),
    on_rotation: &dyn Fn(&str, &str),
    known_project_dirs: &mut Vec<PathBuf>,
) {
    match &event.kind {
        EventKind::Create(_) => {
            for path in &event.paths {
                // Only process .jsonl files
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }

                let path_str = path.to_string_lossy();

                // Detect rotation: new file in a dir that already had tracked files
                // (meaning a previous sibling was removed).
                let is_rotation = file_tracker.get_offset(path).is_none()
                    && path
                        .parent()
                        .is_some_and(|p| file_tracker.has_tracked_files_in_dir(p));

                if is_rotation {
                    let parent_str = path
                        .parent()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    on_rotation(&parent_str, &path_str);
                }

                let messages = file_tracker.read_new_lines(path);
                if !messages.is_empty() {
                    for msg in &messages {
                        if !msg.project_dir.is_empty() {
                            let dir = PathBuf::from(&msg.project_dir);
                            if dir.exists() && !known_project_dirs.contains(&dir) {
                                known_project_dirs.push(dir);
                            }
                        }
                    }
                    on_jsonl(&path_str, messages);
                }
            }
        }
        EventKind::Modify(_) => {
            for path in &event.paths {
                // Only process .jsonl files
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }

                let messages = file_tracker.read_new_lines(path);
                if !messages.is_empty() {
                    for msg in &messages {
                        if !msg.project_dir.is_empty() {
                            let dir = PathBuf::from(&msg.project_dir);
                            if dir.exists() && !known_project_dirs.contains(&dir) {
                                known_project_dirs.push(dir);
                            }
                        }
                    }
                    on_jsonl(&path.to_string_lossy(), messages);
                }
            }
        }
        EventKind::Remove(_) => {
            for path in &event.paths {
                if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    file_tracker.handle_file_removed(path);
                    tracing::info!("Watched file removed: {:?}", path);
                }
            }
        }
        _ => {}
    }
}

/// Scan the base directory for existing project directories.
fn scan_project_dirs(base_dir: &PathBuf, project_dirs: &mut Vec<PathBuf>) {
    if !base_dir.exists() {
        return;
    }

    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Look for .jsonl files to discover project cwds
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub in sub_entries.flatten() {
                        let sub_path = sub.path();
                        if sub_path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                            // Try to read the first line for cwd
                            if let Ok(content) = std::fs::read_to_string(&sub_path) {
                                if let Some(first_line) = content.lines().next() {
                                    if let Ok(obj) =
                                        serde_json::from_str::<serde_json::Value>(first_line)
                                    {
                                        if let Some(cwd) =
                                            obj.get("cwd").and_then(|v| v.as_str())
                                        {
                                            let cwd_path = PathBuf::from(cwd);
                                            if cwd_path.exists()
                                                && !project_dirs.contains(&cwd_path)
                                            {
                                                project_dirs.push(cwd_path);
                                            }
                                        }
                                    }
                                }
                            }
                            break; // One file per project dir is enough
                        }
                    }
                }
            }
        }
    }
}
