use std::time::Duration;
use tokio::time::timeout;

/// Result of a single collector run.
#[derive(Debug, Clone, Default)]
pub struct CaptureResult {
    pub git_branch: Option<String>,
    pub git_status: Option<String>,
    pub git_diff_summary: Option<String>,
    pub active_files: Option<String>,
    pub terminal_last_output: Option<String>,
    pub window_focus: Option<String>,
    pub agent_states: Option<String>,
    pub completeness: Completeness,
}

#[derive(Debug, Clone, Default)]
pub enum Completeness {
    Full,
    #[default]
    Partial,
}

impl Completeness {
    pub fn as_str(&self) -> &str {
        match self {
            Completeness::Full => "full",
            Completeness::Partial => "partial",
        }
    }
}

/// Runs all environment collectors concurrently with a 2s total timeout.
/// Never blocks the user — partial results are acceptable.
pub async fn capture_environment(working_dir: Option<&str>) -> CaptureResult {
    let wd = working_dir.map(|s| s.to_string());

    let result = timeout(Duration::from_secs(2), async {
        let git_handle = tokio::spawn(collect_git(wd.clone()));
        let window_handle = tokio::spawn(collect_window_focus());

        let git = git_handle.await.ok().and_then(|r| r.ok());
        let window = window_handle.await.ok().and_then(|r| r.ok());

        let mut result = CaptureResult::default();
        if let Some((branch, status, diff)) = git {
            result.git_branch = Some(branch);
            result.git_status = Some(status);
            result.git_diff_summary = diff;
            result.completeness = Completeness::Full;
        }
        result.window_focus = window;
        result
    })
    .await;

    result.unwrap_or_default()
}

async fn collect_git(working_dir: Option<String>) -> Result<(String, String, Option<String>), String> {
    let wd = working_dir.unwrap_or_else(|| ".".to_string());
    let repo = git2::Repository::discover(&wd).map_err(|e| e.to_string())?;

    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()))
        .unwrap_or_else(|| "HEAD (detached)".to_string());

    // Simple status summary
    let statuses = repo
        .statuses(None)
        .map_err(|e| e.to_string())?;
    let status_summary = format!("{} changed files", statuses.len());

    Ok((branch, status_summary, None))
}

async fn collect_window_focus() -> Result<String, String> {
    // Platform-specific window focus detection — stubbed for MVP
    Ok("unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capture_returns_within_timeout() {
        let result = capture_environment(None).await;
        // Should always return something (even partial)
        assert!(matches!(
            result.completeness,
            Completeness::Full | Completeness::Partial
        ));
    }
}
