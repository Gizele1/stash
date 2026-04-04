# Security

## Authentication

This is a single-user desktop application. The OS user account provides the access control boundary. There is no application-level authentication layer.

## Authorization

No multi-user authorization model. All data is stored locally in the user's app data directory. OS filesystem permissions are the authorization boundary.

## Secrets Management

- **Storage:** No secrets are stored by this application
- **API keys:** Not applicable — no external API calls at runtime
- **Database:** SQLite file stored in the OS app data directory, protected by OS user permissions (unencrypted)

## Threat Model

| Threat | Mitigation | Status |
|--------|-----------|--------|
| SQL injection | `rusqlite` parameterized queries throughout `db/queries.rs` | In place |
| Path traversal | File watcher reads only from `~/.claude/projects/` (scoped directory) | In place |
| Malicious session file content | Intent extraction uses regex only — no eval, no code execution | In place |
| XSS in frontend | React JSX escaping + Tauri's default strict CSP | In place |
| Tauri command injection | All IPC commands validated by Rust type system at compile time | In place |
| Unintended data exfiltration | No network calls at runtime — all data stays local | In place |

## Data Sensitivity

- Reads Claude Code conversation files from `~/.claude/projects/` — these may contain sensitive prompts and code
- Conversation content is stored in the local SQLite database (`$APP_DATA_DIR/stash.db`)
- Data never leaves the local machine — no telemetry, no analytics, no network calls

## Dependencies

- Security-relevant: `rusqlite` (SQL layer), `git2` (git operations), `notify` (filesystem events)
- Dependency audit: run `cargo audit` (backend) and `npm audit` (frontend) before each release

## Incident Response

- This is a local desktop tool with no network exposure
- If a security issue is found, report it via the GitHub repository's Issues
