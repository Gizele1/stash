use std::process::Command;

use super::PlatformError;

/// Run `git -C {dir} remote get-url origin` to get the remote URL.
pub fn get_remote_url(project_dir: &str) -> Result<String, PlatformError> {
    let output = Command::new("git")
        .args(["-C", project_dir, "remote", "get-url", "origin"])
        .output()
        .map_err(|e| PlatformError::GitError(format!("failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(PlatformError::NoRemote);
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() {
        return Err(PlatformError::NoRemote);
    }
    Ok(url)
}

/// Run `git -C {dir} branch --show-current` to get the current branch name.
pub fn get_current_branch(project_dir: &str) -> Result<String, PlatformError> {
    let output = Command::new("git")
        .args(["-C", project_dir, "branch", "--show-current"])
        .output()
        .map_err(|e| PlatformError::GitError(format!("failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PlatformError::GitError(format!(
            "git branch failed: {}",
            stderr
        )));
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        return Err(PlatformError::GitError(
            "detached HEAD or empty branch name".to_string(),
        ));
    }
    Ok(branch)
}

/// Construct a PR creation URL from a remote URL and branch name.
/// Supports GitHub (github.com) and GitLab (gitlab.com) remotes.
/// Returns None if the remote URL format is not recognized.
pub fn construct_pr_url(remote_url: &str, branch: &str) -> Option<String> {
    let (owner, repo) = parse_remote_url(remote_url)?;

    if remote_url.contains("github.com") {
        Some(format!(
            "https://github.com/{}/{}/pull/new/{}",
            owner, repo, branch
        ))
    } else if remote_url.contains("gitlab.com") {
        Some(format!(
            "https://gitlab.com/{}/{}/-/merge_requests/new?merge_request[source_branch]={}",
            owner, repo, branch
        ))
    } else {
        // Unknown host — try GitHub-style as a best guess
        None
    }
}

/// Parse owner/repo from various remote URL formats:
/// - `https://github.com/owner/repo.git`
/// - `https://github.com/owner/repo`
/// - `git@github.com:owner/repo.git`
/// - `ssh://git@github.com/owner/repo.git`
fn parse_remote_url(url: &str) -> Option<(String, String)> {
    // SSH shorthand: git@host:owner/repo.git
    if let Some(after_colon) = url.strip_prefix("git@") {
        if let Some(colon_pos) = after_colon.find(':') {
            let path = &after_colon[colon_pos + 1..];
            return parse_owner_repo_from_path(path);
        }
    }

    // HTTPS or SSH URL: scheme://host/owner/repo.git
    if url.contains("://") {
        // Find the path after the host
        let without_scheme = url.split("://").nth(1)?;
        let slash_pos = without_scheme.find('/')?;
        let path = &without_scheme[slash_pos + 1..];
        return parse_owner_repo_from_path(path);
    }

    None
}

/// Extract owner/repo from a path like "owner/repo.git" or "owner/repo".
fn parse_owner_repo_from_path(path: &str) -> Option<(String, String)> {
    let clean = path.strip_suffix(".git").unwrap_or(path);
    let parts: Vec<&str> = clean.splitn(3, '/').collect();
    if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_pr_url_github_https() {
        let url =
            construct_pr_url("https://github.com/owner/repo.git", "feature/my-branch");
        assert_eq!(
            url,
            Some("https://github.com/owner/repo/pull/new/feature/my-branch".to_string())
        );
    }

    #[test]
    fn test_construct_pr_url_github_ssh() {
        let url = construct_pr_url("git@github.com:owner/repo.git", "fix/bug-123");
        assert_eq!(
            url,
            Some("https://github.com/owner/repo/pull/new/fix/bug-123".to_string())
        );
    }

    #[test]
    fn test_construct_pr_url_gitlab_https() {
        let url = construct_pr_url("https://gitlab.com/owner/repo.git", "feature/x");
        assert_eq!(
            url,
            Some("https://gitlab.com/owner/repo/-/merge_requests/new?merge_request[source_branch]=feature/x".to_string())
        );
    }

    #[test]
    fn test_construct_pr_url_gitlab_ssh() {
        let url = construct_pr_url("git@gitlab.com:owner/repo.git", "main");
        assert_eq!(
            url,
            Some("https://gitlab.com/owner/repo/-/merge_requests/new?merge_request[source_branch]=main".to_string())
        );
    }

    #[test]
    fn test_construct_pr_url_unknown_host() {
        let url = construct_pr_url("https://bitbucket.org/owner/repo.git", "main");
        assert_eq!(url, None);
    }

    #[test]
    fn test_construct_pr_url_no_git_suffix() {
        let url = construct_pr_url("https://github.com/owner/repo", "develop");
        assert_eq!(
            url,
            Some("https://github.com/owner/repo/pull/new/develop".to_string())
        );
    }

    #[test]
    fn test_parse_remote_url_ssh_with_scheme() {
        let result = parse_remote_url("ssh://git@github.com/owner/repo.git");
        assert_eq!(
            result,
            Some(("owner".to_string(), "repo".to_string()))
        );
    }

    #[test]
    fn test_parse_remote_url_invalid() {
        assert_eq!(parse_remote_url("not-a-url"), None);
        assert_eq!(parse_remote_url(""), None);
    }
}
