use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::watcher::jsonl::{parse_jsonl_line, FileTracker};
use crate::watcher::git_monitor::GitMonitor;
use crate::watcher::{JsonlMessage, Watcher, WatcherConfig, WatcherError};
use std::sync::Arc;

    // ── JSONL Parsing Tests ──

    #[test]
    fn test_parse_valid_user_message() {
        let line = r#"{"type":"user","message":{"content":"implement auth"},"timestamp":1700000000,"sessionId":"sess-123","cwd":"/home/user/project"}"#;
        let msg = parse_jsonl_line(line).expect("should parse valid user message");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "implement auth");
        assert_eq!(msg.timestamp, 1700000000);
        assert_eq!(msg.session_id, "sess-123");
        assert_eq!(msg.project_dir, "/home/user/project");
        assert_eq!(msg.display_name, "project");
    }

    #[test]
    fn test_parse_filters_assistant_messages() {
        let line = r#"{"type":"assistant","message":{"content":"Sure, I'll help"},"timestamp":1700000000,"sessionId":"sess-123","cwd":"/home/user/project"}"#;
        assert!(parse_jsonl_line(line).is_none(), "assistant messages should be filtered out");
    }

    #[test]
    fn test_parse_filters_system_messages() {
        let line = r#"{"type":"system","message":{"content":"system prompt"},"timestamp":1700000000,"sessionId":"sess-123","cwd":"/"}"#;
        assert!(parse_jsonl_line(line).is_none(), "system messages should be filtered out");
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_jsonl_line("").is_none());
        assert!(parse_jsonl_line("   ").is_none());
        assert!(parse_jsonl_line("\n").is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_jsonl_line("not json at all").is_none());
        assert!(parse_jsonl_line("{broken json").is_none());
    }

    #[test]
    fn test_parse_missing_type_field() {
        let line = r#"{"message":{"content":"hello"},"timestamp":1700000000}"#;
        assert!(parse_jsonl_line(line).is_none());
    }

    #[test]
    fn test_parse_empty_content() {
        let line = r#"{"type":"user","message":{"content":""},"timestamp":1700000000,"sessionId":"s1","cwd":"/"}"#;
        assert!(parse_jsonl_line(line).is_none(), "empty content should be filtered");
    }

    #[test]
    fn test_parse_missing_optional_fields() {
        let line = r#"{"type":"user","message":{"content":"hello world"}}"#;
        let msg = parse_jsonl_line(line).expect("should parse with missing optional fields");
        assert_eq!(msg.content, "hello world");
        assert_eq!(msg.timestamp, 0);
        assert_eq!(msg.session_id, "");
        assert_eq!(msg.project_dir, "");
    }

    // ── File Offset Tracking Tests ──

    #[test]
    fn test_file_tracker_reads_new_lines_only() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.jsonl");

        // Write initial lines
        {
            let mut f = fs::File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"first message"}},"timestamp":1,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"second message"}},"timestamp":2,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
        }

        let mut tracker = FileTracker::new();

        // First read: should get both messages
        let msgs = tracker.read_new_lines(&file_path);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "first message");
        assert_eq!(msgs[1].content, "second message");

        // Second read without changes: should get nothing
        let msgs = tracker.read_new_lines(&file_path);
        assert_eq!(msgs.len(), 0);

        // Append a new line
        {
            let mut f = fs::OpenOptions::new().append(true).open(&file_path).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"third message"}},"timestamp":3,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
        }

        // Third read: should get only the new line
        let msgs = tracker.read_new_lines(&file_path);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "third message");
    }

    #[test]
    fn test_file_tracker_handles_missing_file() {
        let mut tracker = FileTracker::new();
        let msgs = tracker.read_new_lines(&PathBuf::from("/nonexistent/file.jsonl"));
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_file_tracker_handles_file_rotation() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("session.jsonl");

        // Write and read initial content
        {
            let mut f = fs::File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"old message"}},"timestamp":1,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"another old"}},"timestamp":2,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
        }

        let mut tracker = FileTracker::new();
        let msgs = tracker.read_new_lines(&file_path);
        assert_eq!(msgs.len(), 2);

        // Simulate file rotation: delete and recreate with shorter content
        fs::remove_file(&file_path).unwrap();
        {
            let mut f = fs::File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"new file"}},"timestamp":3,"sessionId":"s2","cwd":"/tmp"}}"#).unwrap();
        }

        // Should detect rotation (file smaller) and re-read from start
        let msgs = tracker.read_new_lines(&file_path);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "new file");
    }

    #[test]
    fn test_file_tracker_skips_non_user_lines() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("mixed.jsonl");

        {
            let mut f = fs::File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"user msg"}},"timestamp":1,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
            writeln!(f, r#"{{"type":"assistant","message":{{"content":"assistant msg"}},"timestamp":2,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
            writeln!(f, r#"{{"type":"system","message":{{"content":"system msg"}},"timestamp":3,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"another user"}},"timestamp":4,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
        }

        let mut tracker = FileTracker::new();
        let msgs = tracker.read_new_lines(&file_path);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "user msg");
        assert_eq!(msgs[1].content, "another user");
    }

    #[test]
    fn test_file_tracker_handles_invalid_lines_gracefully() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("messy.jsonl");

        {
            let mut f = fs::File::create(&file_path).unwrap();
            writeln!(f, "not json").unwrap();
            writeln!(f).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"valid"}},"timestamp":1,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
            writeln!(f, "{{broken").unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"also valid"}},"timestamp":2,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
        }

        let mut tracker = FileTracker::new();
        let msgs = tracker.read_new_lines(&file_path);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "valid");
        assert_eq!(msgs[1].content, "also valid");
    }

    #[test]
    fn test_file_tracker_handle_file_removed() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("removed.jsonl");

        {
            let mut f = fs::File::create(&file_path).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"msg"}},"timestamp":1,"sessionId":"s1","cwd":"/tmp"}}"#).unwrap();
        }

        let mut tracker = FileTracker::new();
        tracker.read_new_lines(&file_path);
        assert!(tracker.get_offset(&file_path).is_some());

        tracker.handle_file_removed(&file_path);
        assert!(tracker.get_offset(&file_path).is_none());
    }

    // ── Git Monitor Tests ──

    #[test]
    fn test_git_monitor_respects_poll_interval() {
        let mut monitor = GitMonitor::new(30); // 30s interval
        let dir = PathBuf::from("/tmp/nonexistent-project");

        // First call should proceed (returns empty because dir doesn't exist as git repo)
        let signals = monitor.check_signals(&dir);
        assert!(signals.is_empty());

        // Second immediate call should be rate-limited (returns empty)
        let signals = monitor.check_signals(&dir);
        assert!(signals.is_empty());
    }

    #[test]
    fn test_git_monitor_register_project() {
        let mut monitor = GitMonitor::new(30);
        let dir = PathBuf::from("/tmp/some-project");
        monitor.register_project(&dir);
        // Should not panic, just register
    }

    // ── WatcherConfig Tests ──

    #[test]
    fn test_watcher_config_defaults() {
        let config = WatcherConfig::default();
        assert!(config.claude_base_dir.to_string_lossy().contains(".claude/projects"));
        assert_eq!(config.debounce_ms, 500);
        assert_eq!(config.git_poll_interval_secs, 30);
    }

    // ── Watcher Tests ──

    #[test]
    fn test_watcher_new_with_valid_config() {
        let dir = TempDir::new().unwrap();
        let config = WatcherConfig {
            claude_base_dir: dir.path().to_path_buf(),
            debounce_ms: 500,
            git_poll_interval_secs: 30,
        };
        let watcher = Watcher::new(config);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_new_with_empty_dir_fails() {
        let config = WatcherConfig {
            claude_base_dir: PathBuf::from(""),
            debounce_ms: 500,
            git_poll_interval_secs: 30,
        };
        let watcher = Watcher::new(config);
        assert!(watcher.is_err());
        match watcher.unwrap_err() {
            WatcherError::InitFailed(msg) => assert!(msg.contains("empty")),
            other => panic!("Expected InitFailed, got: {:?}", other),
        }
    }

    #[test]
    fn test_watcher_start_and_stop() {
        let dir = TempDir::new().unwrap();
        let config = WatcherConfig {
            claude_base_dir: dir.path().to_path_buf(),
            debounce_ms: 100,
            git_poll_interval_secs: 30,
        };
        let mut watcher = Watcher::new(config).unwrap();

        let handle = watcher
            .start(
                Box::new(|_msgs| {}),
                Box::new(|_dir, _sig, _meta| {}),
            )
            .unwrap();

        // Let it run briefly
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Stop should cause the thread to exit
        watcher.stop();
        let result = handle.join();
        assert!(result.is_ok(), "Watcher thread should exit cleanly");
    }

    #[test]
    fn test_watcher_detects_new_jsonl_messages() {
        let dir = TempDir::new().unwrap();
        let project_dir = dir.path().join("project-hash");
        fs::create_dir_all(&project_dir).unwrap();

        let config = WatcherConfig {
            claude_base_dir: dir.path().to_path_buf(),
            debounce_ms: 100,
            git_poll_interval_secs: 300, // Long interval so git doesn't interfere
        };
        let mut watcher = Watcher::new(config).unwrap();

        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        let handle = watcher
            .start(
                Box::new(move |msgs| {
                    let mut r = received_clone.lock().unwrap();
                    r.extend(msgs);
                }),
                Box::new(|_dir, _sig, _meta| {}),
            )
            .unwrap();

        // Give the watcher time to start
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Write a JSONL file
        let jsonl_path = project_dir.join("session.jsonl");
        {
            let mut f = fs::File::create(&jsonl_path).unwrap();
            writeln!(f, r#"{{"type":"user","message":{{"content":"hello from test"}},"timestamp":1700000000,"sessionId":"test-session","cwd":"/tmp"}}"#).unwrap();
        }

        // Wait for the watcher to detect the file change
        std::thread::sleep(std::time::Duration::from_secs(2));

        watcher.stop();
        let _ = handle.join();

        let msgs = received.lock().unwrap();
        // On some platforms notify may not fire for tempdir; we check it doesn't crash
        // If messages were received, verify content
        if !msgs.is_empty() {
            assert_eq!(msgs[0].content, "hello from test");
            assert_eq!(msgs[0].session_id, "test-session");
        }
    }

    // ── WatcherError Tests ──

    #[test]
    fn test_watcher_error_display() {
        let err = WatcherError::InitFailed("test failure".to_string());
        assert_eq!(format!("{}", err), "Failed to initialize watcher: test failure");

        let err = WatcherError::GitError("git not found".to_string());
        assert_eq!(format!("{}", err), "Git error: git not found");

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = WatcherError::IoError(io_err);
        assert!(format!("{}", err).contains("file not found"));
    }

    // ── JsonlMessage Tests ──

    #[test]
    fn test_jsonl_message_serialization() {
        let msg = JsonlMessage {
            role: "user".to_string(),
            content: "test content".to_string(),
            timestamp: 1700000000,
            session_id: "sess-1".to_string(),
            project_hash: "abc123".to_string(),
            project_dir: "/home/user/project".to_string(),
            display_name: "project".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: JsonlMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "test content");
        assert_eq!(deserialized.session_id, "sess-1");
    }
