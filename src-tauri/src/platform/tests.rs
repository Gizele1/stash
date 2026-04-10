use super::*;
use std::sync::{Arc, Mutex};

// ── Mock PlatformBridge ──

#[derive(Debug, Clone)]
struct MockCall {
    method: String,
    args: Vec<String>,
}

struct MockPlatformBridge {
    calls: Mutex<Vec<MockCall>>,
    /// Map of project_dir -> window_id for find_terminal_window
    terminal_windows: Mutex<Vec<(String, u64)>>,
    /// Registered hotkeys — set of key_combo strings that are "taken"
    registered_hotkeys: Mutex<Vec<String>>,
    /// Hotkeys that should conflict
    conflicting_hotkeys: Mutex<Vec<String>>,
    /// Whether setup_pet_window should fail
    setup_should_fail: Mutex<bool>,
}

impl MockPlatformBridge {
    fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            terminal_windows: Mutex::new(Vec::new()),
            registered_hotkeys: Mutex::new(Vec::new()),
            conflicting_hotkeys: Mutex::new(Vec::new()),
            setup_should_fail: Mutex::new(false),
        }
    }

    fn with_terminal(self, project_dir: &str, window_id: u64) -> Self {
        self.terminal_windows
            .lock()
            .unwrap()
            .push((project_dir.to_string(), window_id));
        self
    }

    fn with_conflicting_hotkey(self, key_combo: &str) -> Self {
        self.conflicting_hotkeys
            .lock()
            .unwrap()
            .push(key_combo.to_string());
        self
    }

    fn with_setup_failure(self) -> Self {
        *self.setup_should_fail.lock().unwrap() = true;
        self
    }

    fn call_count(&self, method: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.method == method)
            .count()
    }

    fn was_called(&self, method: &str) -> bool {
        self.call_count(method) > 0
    }
}

impl PlatformBridge for MockPlatformBridge {
    fn setup_pet_window(&self, window_id: u64) -> Result<(), PlatformError> {
        self.calls.lock().unwrap().push(MockCall {
            method: "setup_pet_window".to_string(),
            args: vec![window_id.to_string()],
        });
        if *self.setup_should_fail.lock().unwrap() {
            Err(PlatformError::X11Error("mock failure".to_string()))
        } else {
            Ok(())
        }
    }

    fn set_click_through(
        &self,
        window_id: u64,
        passthrough_regions: &[Rect],
    ) -> Result<(), PlatformError> {
        self.calls.lock().unwrap().push(MockCall {
            method: "set_click_through".to_string(),
            args: vec![
                window_id.to_string(),
                format!("{} regions", passthrough_regions.len()),
            ],
        });
        Ok(())
    }

    fn find_terminal_window(&self, project_dir: &str) -> Result<Option<u64>, PlatformError> {
        self.calls.lock().unwrap().push(MockCall {
            method: "find_terminal_window".to_string(),
            args: vec![project_dir.to_string()],
        });

        let terminals = self.terminal_windows.lock().unwrap();
        for (dir, wid) in terminals.iter() {
            if dir == project_dir {
                return Ok(Some(*wid));
            }
        }
        Ok(None)
    }

    fn focus_window(&self, window_id: u64) -> Result<(), PlatformError> {
        self.calls.lock().unwrap().push(MockCall {
            method: "focus_window".to_string(),
            args: vec![window_id.to_string()],
        });
        Ok(())
    }

    fn register_hotkey(&self, key_combo: &str, action: &str) -> Result<bool, PlatformError> {
        self.calls.lock().unwrap().push(MockCall {
            method: "register_hotkey".to_string(),
            args: vec![key_combo.to_string(), action.to_string()],
        });

        let conflicts = self.conflicting_hotkeys.lock().unwrap();
        if conflicts.contains(&key_combo.to_string()) {
            return Err(PlatformError::HotkeyConflict);
        }

        self.registered_hotkeys
            .lock()
            .unwrap()
            .push(key_combo.to_string());
        Ok(true)
    }
}

fn make_service(bridge: MockPlatformBridge) -> (PlatformService, Arc<MockPlatformBridge>) {
    let db = Arc::new(crate::db::Database::in_memory().unwrap());
    let bridge = Arc::new(bridge);
    let bridge_clone = bridge.clone();
    // We need to move the Arc into a Box<dyn PlatformBridge>
    // To do this, we implement PlatformBridge for Arc<MockPlatformBridge>
    let service = PlatformService {
        bridge: Box::new(ArcBridge(bridge_clone)),
        db,
    };
    (service, bridge)
}

/// Wrapper to let us use Arc<MockPlatformBridge> as Box<dyn PlatformBridge>
struct ArcBridge(Arc<MockPlatformBridge>);

impl PlatformBridge for ArcBridge {
    fn setup_pet_window(&self, window_id: u64) -> Result<(), PlatformError> {
        self.0.setup_pet_window(window_id)
    }
    fn set_click_through(
        &self,
        window_id: u64,
        passthrough_regions: &[Rect],
    ) -> Result<(), PlatformError> {
        self.0.set_click_through(window_id, passthrough_regions)
    }
    fn find_terminal_window(&self, project_dir: &str) -> Result<Option<u64>, PlatformError> {
        self.0.find_terminal_window(project_dir)
    }
    fn focus_window(&self, window_id: u64) -> Result<(), PlatformError> {
        self.0.focus_window(window_id)
    }
    fn register_hotkey(&self, key_combo: &str, action: &str) -> Result<bool, PlatformError> {
        self.0.register_hotkey(key_combo, action)
    }
}

// ── Tests ──

#[test]
fn focus_terminal_success_when_found() {
    let mock = MockPlatformBridge::new().with_terminal("/home/user/project", 12345);
    let (service, bridge) = make_service(mock);

    let result = service.focus_terminal("/home/user/project").unwrap();

    assert_eq!(
        result,
        FocusResult {
            success: true,
            fallback_used: false
        }
    );
    assert!(bridge.was_called("find_terminal_window"));
    assert!(bridge.was_called("focus_window"));
}

#[test]
fn focus_terminal_fallback_when_parent_matches() {
    // Register terminal for parent dir, not exact project dir
    let mock = MockPlatformBridge::new().with_terminal("/home/user", 54321);
    let (service, bridge) = make_service(mock);

    let result = service
        .focus_terminal("/home/user/project")
        .unwrap();

    assert_eq!(
        result,
        FocusResult {
            success: true,
            fallback_used: true
        }
    );
    assert_eq!(bridge.call_count("find_terminal_window"), 2); // exact + fuzzy
    assert!(bridge.was_called("focus_window"));
}

#[test]
fn focus_terminal_failure_when_not_found() {
    let mock = MockPlatformBridge::new(); // no terminals registered
    let (service, bridge) = make_service(mock);

    let result = service.focus_terminal("/home/user/project").unwrap();

    assert_eq!(
        result,
        FocusResult {
            success: false,
            fallback_used: false
        }
    );
    assert!(!bridge.was_called("focus_window")); // should not attempt focus
}

#[test]
fn register_hotkey_success() {
    let mock = MockPlatformBridge::new();
    let (service, _bridge) = make_service(mock);

    let result = service.register_hotkey("Ctrl+Shift+T", "toggle_panel").unwrap();
    assert!(result);
}

#[test]
fn register_hotkey_detects_conflict() {
    let mock = MockPlatformBridge::new().with_conflicting_hotkey("Ctrl+Shift+T");
    let (service, _bridge) = make_service(mock);

    let result = service.register_hotkey("Ctrl+Shift+T", "toggle_panel");
    assert!(matches!(result, Err(PlatformError::HotkeyConflict)));
}

#[test]
fn setup_pet_window_calls_bridge() {
    let mock = MockPlatformBridge::new();
    let (service, bridge) = make_service(mock);

    service.setup_pet_window(99999).unwrap();
    assert!(bridge.was_called("setup_pet_window"));
}

#[test]
fn setup_pet_window_propagates_error() {
    let mock = MockPlatformBridge::new().with_setup_failure();
    let (service, _bridge) = make_service(mock);

    let result = service.setup_pet_window(99999);
    assert!(matches!(result, Err(PlatformError::X11Error(_))));
}

#[test]
fn set_click_through_empty_regions_full_passthrough() {
    let mock = MockPlatformBridge::new();
    let (service, bridge) = make_service(mock);

    service.set_click_through(12345, &[]).unwrap();
    assert!(bridge.was_called("set_click_through"));

    let calls = bridge.calls.lock().unwrap();
    let call = calls.iter().find(|c| c.method == "set_click_through").unwrap();
    assert_eq!(call.args[1], "0 regions");
}

#[test]
fn set_click_through_with_regions() {
    let mock = MockPlatformBridge::new();
    let (service, bridge) = make_service(mock);

    let regions = vec![
        Rect { x: 10, y: 20, width: 100, height: 50 },
        Rect { x: 200, y: 300, width: 80, height: 40 },
    ];

    service.set_click_through(12345, &regions).unwrap();

    let calls = bridge.calls.lock().unwrap();
    let call = calls.iter().find(|c| c.method == "set_click_through").unwrap();
    assert_eq!(call.args[1], "2 regions");
}

#[test]
fn save_and_get_pet_position() {
    let db = Arc::new(crate::db::Database::in_memory().unwrap());
    let mock = MockPlatformBridge::new();
    let service = PlatformService::new(Box::new(mock), db);

    // Initially no position
    let pos = service.get_pet_position().unwrap();
    assert_eq!(pos, None);

    // Save position
    service.save_pet_position(100, 200).unwrap();

    // Read back
    let pos = service.get_pet_position().unwrap();
    assert_eq!(pos, Some((100, 200)));

    // Update position
    service.save_pet_position(-50, 300).unwrap();
    let pos = service.get_pet_position().unwrap();
    assert_eq!(pos, Some((-50, 300)));
}

#[test]
fn open_pr_url_github_https() {
    let url = git_ops::construct_pr_url(
        "https://github.com/myorg/myrepo.git",
        "feature/cool-stuff",
    );
    assert_eq!(
        url,
        Some("https://github.com/myorg/myrepo/pull/new/feature/cool-stuff".to_string())
    );
}

#[test]
fn open_pr_url_github_ssh() {
    let url = git_ops::construct_pr_url(
        "git@github.com:myorg/myrepo.git",
        "fix/issue-42",
    );
    assert_eq!(
        url,
        Some("https://github.com/myorg/myrepo/pull/new/fix/issue-42".to_string())
    );
}

#[test]
fn open_pr_url_gitlab() {
    let url = git_ops::construct_pr_url(
        "https://gitlab.com/group/project.git",
        "feature/x",
    );
    assert_eq!(
        url,
        Some("https://gitlab.com/group/project/-/merge_requests/new?merge_request[source_branch]=feature/x".to_string())
    );
}

#[test]
fn open_pr_url_unknown_host_returns_none() {
    let url = git_ops::construct_pr_url(
        "https://selfhosted.example.com/owner/repo.git",
        "main",
    );
    assert_eq!(url, None);
}

#[test]
fn open_pr_url_no_git_suffix() {
    let url = git_ops::construct_pr_url(
        "https://github.com/owner/repo",
        "develop",
    );
    assert_eq!(
        url,
        Some("https://github.com/owner/repo/pull/new/develop".to_string())
    );
}

#[test]
fn platform_error_display() {
    let err = PlatformError::X11Error("test error".to_string());
    assert_eq!(err.to_string(), "X11 error: test error");

    let err = PlatformError::WindowNotFound;
    assert_eq!(err.to_string(), "Window not found");

    let err = PlatformError::TerminalNotFound;
    assert_eq!(err.to_string(), "Terminal not found");

    let err = PlatformError::HotkeyConflict;
    assert_eq!(err.to_string(), "Hotkey conflict");

    let err = PlatformError::NoPrFound;
    assert_eq!(err.to_string(), "No PR found");

    let err = PlatformError::NoRemote;
    assert_eq!(err.to_string(), "No remote configured");

    let err = PlatformError::GitError("bad git".to_string());
    assert_eq!(err.to_string(), "Git error: bad git");

    let err = PlatformError::ConfigError("db fail".to_string());
    assert_eq!(err.to_string(), "Config error: db fail");
}

#[test]
fn platform_error_converts_to_string() {
    let err = PlatformError::TerminalNotFound;
    let s: String = err.into();
    assert_eq!(s, "Terminal not found");
}

#[test]
fn rect_serialization() {
    let rect = Rect {
        x: 10,
        y: 20,
        width: 100,
        height: 50,
    };
    let json = serde_json::to_string(&rect).unwrap();
    let deserialized: Rect = serde_json::from_str(&json).unwrap();
    assert_eq!(rect, deserialized);
}

#[test]
fn focus_result_serialization() {
    let result = FocusResult {
        success: true,
        fallback_used: false,
    };
    let json = serde_json::to_string(&result).unwrap();
    let deserialized: FocusResult = serde_json::from_str(&json).unwrap();
    assert_eq!(result, deserialized);
}

#[test]
fn pr_url_result_serialization() {
    let result = PrUrlResult {
        url: Some("https://github.com/o/r/pull/new/main".to_string()),
        opened: true,
    };
    let json = serde_json::to_string(&result).unwrap();
    let deserialized: PrUrlResult = serde_json::from_str(&json).unwrap();
    assert_eq!(result, deserialized);
}

#[test]
fn focus_terminal_root_path_no_parent_fallback() {
    // Edge case: root path "/" has no meaningful parent
    let mock = MockPlatformBridge::new();
    let (service, bridge) = make_service(mock);

    let result = service.focus_terminal("/").unwrap();

    assert_eq!(
        result,
        FocusResult {
            success: false,
            fallback_used: false
        }
    );
    // Should have tried exact match, then parent "/" which is same → skip or fail
    // find_terminal_window called at least once for exact match
    assert!(bridge.was_called("find_terminal_window"));
}
