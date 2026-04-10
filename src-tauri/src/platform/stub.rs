use super::{PlatformBridge, PlatformError, Rect};

/// A stub platform bridge for environments without X11 (e.g., WSL, CI, Wayland).
/// All operations succeed as no-ops.
pub struct StubPlatformBridge;

impl StubPlatformBridge {
    pub fn new() -> Self {
        Self
    }
}

impl Default for StubPlatformBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformBridge for StubPlatformBridge {
    fn setup_pet_window(&self, _window_id: u64) -> Result<(), PlatformError> {
        tracing::debug!("StubPlatformBridge: setup_pet_window (no-op)");
        Ok(())
    }

    fn set_click_through(
        &self,
        _window_id: u64,
        _passthrough_regions: &[Rect],
    ) -> Result<(), PlatformError> {
        tracing::debug!("StubPlatformBridge: set_click_through (no-op)");
        Ok(())
    }

    fn find_terminal_window(&self, _project_dir: &str) -> Result<Option<u64>, PlatformError> {
        Ok(None)
    }

    fn focus_window(&self, _window_id: u64) -> Result<(), PlatformError> {
        tracing::debug!("StubPlatformBridge: focus_window (no-op)");
        Ok(())
    }

    fn register_hotkey(&self, _key_combo: &str, _action: &str) -> Result<bool, PlatformError> {
        tracing::debug!("StubPlatformBridge: register_hotkey (no-op)");
        Ok(false)
    }
}
