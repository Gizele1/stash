pub mod git_ops;
pub mod x11_bridge;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use crate::db::Database;

// ── Error Types ──

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("X11 error: {0}")]
    X11Error(String),
    #[error("Window not found")]
    WindowNotFound,
    #[error("Terminal not found")]
    TerminalNotFound,
    #[error("Hotkey conflict")]
    HotkeyConflict,
    #[error("No PR found")]
    NoPrFound,
    #[error("No remote configured")]
    NoRemote,
    #[error("Git error: {0}")]
    GitError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
}

impl From<PlatformError> for String {
    fn from(e: PlatformError) -> String {
        e.to_string()
    }
}

// ── Data Types ──

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FocusResult {
    pub success: bool,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrUrlResult {
    pub url: Option<String>,
    pub opened: bool,
}

// ── PlatformBridge Trait ──

pub trait PlatformBridge: Send + Sync {
    fn setup_pet_window(&self, window_id: u64) -> Result<(), PlatformError>;
    fn set_click_through(
        &self,
        window_id: u64,
        passthrough_regions: &[Rect],
    ) -> Result<(), PlatformError>;
    fn find_terminal_window(&self, project_dir: &str) -> Result<Option<u64>, PlatformError>;
    fn focus_window(&self, window_id: u64) -> Result<(), PlatformError>;
    fn register_hotkey(&self, key_combo: &str, action: &str) -> Result<bool, PlatformError>;
}

// ── Terminal info for matching ──

#[derive(Debug, Clone)]
pub struct TerminalWindow {
    pub window_id: u64,
    pub cwd: Option<String>,
}

// ── PlatformService ──

pub struct PlatformService {
    bridge: Box<dyn PlatformBridge>,
    db: Arc<Database>,
}

impl PlatformService {
    pub fn new(bridge: Box<dyn PlatformBridge>, db: Arc<Database>) -> Self {
        Self { bridge, db }
    }

    /// Focus the terminal window that has the given project directory open.
    /// Falls back to fuzzy (parent directory) matching if exact match fails.
    pub fn focus_terminal(&self, project_dir: &str) -> Result<FocusResult, PlatformError> {
        // Try exact match first
        match self.bridge.find_terminal_window(project_dir)? {
            Some(window_id) => {
                self.bridge.focus_window(window_id)?;
                Ok(FocusResult {
                    success: true,
                    fallback_used: false,
                })
            }
            None => {
                // Try fuzzy match: check parent directory
                if let Some(parent) = std::path::Path::new(project_dir).parent() {
                    if let Some(parent_str) = parent.to_str() {
                        if let Some(window_id) =
                            self.bridge.find_terminal_window(parent_str)?
                        {
                            self.bridge.focus_window(window_id)?;
                            return Ok(FocusResult {
                                success: true,
                                fallback_used: true,
                            });
                        }
                    }
                }
                Ok(FocusResult {
                    success: false,
                    fallback_used: false,
                })
            }
        }
    }

    /// Open the PR creation URL for the current branch of the given project directory.
    pub fn open_pr_url(&self, project_dir: &str) -> Result<PrUrlResult, PlatformError> {
        let remote_url = match git_ops::get_remote_url(project_dir) {
            Ok(url) => url,
            Err(_) => {
                return Err(PlatformError::NoRemote);
            }
        };

        let branch = match git_ops::get_current_branch(project_dir) {
            Ok(b) => b,
            Err(e) => {
                return Err(PlatformError::GitError(format!(
                    "failed to get current branch: {}",
                    e
                )));
            }
        };

        let url = git_ops::construct_pr_url(&remote_url, &branch);

        match url {
            Some(ref u) => {
                let opened = open::that(u).is_ok();
                Ok(PrUrlResult {
                    url: url.clone(),
                    opened,
                })
            }
            None => Err(PlatformError::NoPrFound),
        }
    }

    /// Save pet window position to the database config table.
    pub fn save_pet_position(&self, x: i32, y: i32) -> Result<(), PlatformError> {
        let conn = self.db.conn();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .map_err(|e| PlatformError::ConfigError(e.to_string()))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params!["pet_x", x.to_string()],
        )
        .map_err(|e| PlatformError::ConfigError(e.to_string()))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params!["pet_y", y.to_string()],
        )
        .map_err(|e| PlatformError::ConfigError(e.to_string()))?;

        Ok(())
    }

    /// Read pet window position from the database config table.
    pub fn get_pet_position(&self) -> Result<Option<(i32, i32)>, PlatformError> {
        let conn = self.db.conn();

        // Check if config table exists
        let table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='config'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .unwrap_or(false);

        if !table_exists {
            return Ok(None);
        }

        let x: Option<String> = conn
            .query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params!["pet_x"],
                |row| row.get(0),
            )
            .ok();

        let y: Option<String> = conn
            .query_row(
                "SELECT value FROM config WHERE key = ?1",
                rusqlite::params!["pet_y"],
                |row| row.get(0),
            )
            .ok();

        match (x, y) {
            (Some(x_str), Some(y_str)) => {
                let x_val = x_str
                    .parse::<i32>()
                    .map_err(|e| PlatformError::ConfigError(e.to_string()))?;
                let y_val = y_str
                    .parse::<i32>()
                    .map_err(|e| PlatformError::ConfigError(e.to_string()))?;
                Ok(Some((x_val, y_val)))
            }
            _ => Ok(None),
        }
    }

    /// Register a global hotkey.
    pub fn register_hotkey(
        &self,
        key_combo: &str,
        action: &str,
    ) -> Result<bool, PlatformError> {
        self.bridge.register_hotkey(key_combo, action)
    }

    /// Set up the pet window with appropriate X11 properties.
    pub fn setup_pet_window(&self, window_id: u64) -> Result<(), PlatformError> {
        self.bridge.setup_pet_window(window_id)
    }

    /// Set click-through regions on a window.
    pub fn set_click_through(
        &self,
        window_id: u64,
        regions: &[Rect],
    ) -> Result<(), PlatformError> {
        self.bridge.set_click_through(window_id, regions)
    }
}
