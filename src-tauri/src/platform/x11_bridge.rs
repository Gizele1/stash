use super::{PlatformBridge, PlatformError, Rect};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// Known terminal emulator WM_CLASS values.
const TERMINAL_CLASSES: &[&str] = &[
    "gnome-terminal",
    "gnome-terminal-server",
    "konsole",
    "alacritty",
    "kitty",
    "xterm",
    "urxvt",
    "terminator",
    "tilix",
    "wezterm",
];

/// Real X11 bridge that communicates with the X11 display server.
pub struct X11Bridge {
    conn: RustConnection,
    root: u32,
}

impl X11Bridge {
    /// Create a new X11Bridge. Returns an error if no X11 display is available.
    pub fn new() -> Result<Self, PlatformError> {
        let (conn, screen_num) = RustConnection::connect(None)
            .map_err(|e| PlatformError::X11Error(format!("failed to connect to X11: {}", e)))?;

        let root = conn.setup().roots[screen_num].root;

        Ok(Self { conn, root })
    }

    /// Get an atom by name (intern_atom).
    fn atom(&self, name: &str) -> Result<u32, PlatformError> {
        let reply = self
            .conn
            .intern_atom(false, name.as_bytes())
            .map_err(|e| PlatformError::X11Error(format!("intern_atom request failed: {}", e)))?
            .reply()
            .map_err(|e| PlatformError::X11Error(format!("intern_atom reply failed: {}", e)))?;
        Ok(reply.atom)
    }

    /// Send a _NET_WM_STATE client message.
    fn send_wm_state_message(
        &self,
        window: u32,
        action: u32,
        property: u32,
    ) -> Result<(), PlatformError> {
        let wm_state = self.atom("_NET_WM_STATE")?;

        let event = ClientMessageEvent::new(
            32,
            window,
            wm_state,
            [action, property, 0, 1, 0], // source indication = 1 (normal app)
        );

        self.conn
            .send_event(
                false,
                self.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            )
            .map_err(|e| PlatformError::X11Error(format!("send_event failed: {}", e)))?;

        self.conn
            .flush()
            .map_err(|e| PlatformError::X11Error(format!("flush failed: {}", e)))?;

        Ok(())
    }

    /// Read a window's _NET_WM_PID property.
    fn get_window_pid(&self, window: u32) -> Option<u32> {
        let atom = self.atom("_NET_WM_PID").ok()?;
        let reply = self
            .conn
            .get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)
            .ok()?
            .reply()
            .ok()?;

        if reply.value_len == 1 && reply.format == 32 {
            let data = reply.value32()?.next()?;
            Some(data)
        } else {
            None
        }
    }

    /// Read a window's WM_CLASS property.
    fn get_wm_class(&self, window: u32) -> Option<String> {
        let reply = self
            .conn
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 256)
            .ok()?
            .reply()
            .ok()?;

        let raw = String::from_utf8_lossy(&reply.value).to_lowercase();
        Some(raw)
    }

    /// Check if a window is a terminal based on WM_CLASS.
    fn is_terminal_window(&self, window: u32) -> bool {
        if let Some(wm_class) = self.get_wm_class(window) {
            TERMINAL_CLASSES
                .iter()
                .any(|terminal| wm_class.contains(terminal))
        } else {
            false
        }
    }

    /// Read /proc/{pid}/cwd to determine the working directory of a process.
    fn read_process_cwd(pid: u32) -> Option<String> {
        let link_path = format!("/proc/{}/cwd", pid);
        std::fs::read_link(&link_path)
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
    }

    /// Enumerate all top-level windows via _NET_CLIENT_LIST.
    fn get_client_list(&self) -> Result<Vec<u32>, PlatformError> {
        let atom = self.atom("_NET_CLIENT_LIST")?;
        let reply = self
            .conn
            .get_property(false, self.root, atom, AtomEnum::WINDOW, 0, u32::MAX)
            .map_err(|e| PlatformError::X11Error(format!("get_property failed: {}", e)))?
            .reply()
            .map_err(|e| PlatformError::X11Error(format!("get_property reply failed: {}", e)))?;

        if reply.format == 32 {
            Ok(reply.value32().map(|iter| iter.collect()).unwrap_or_default())
        } else {
            Ok(Vec::new())
        }
    }
}

impl PlatformBridge for X11Bridge {
    fn setup_pet_window(&self, window_id: u64) -> Result<(), PlatformError> {
        let wid = window_id as u32;

        // Set window type to DOCK
        let wm_type = self.atom("_NET_WM_WINDOW_TYPE")?;
        let dock_type = self.atom("_NET_WM_WINDOW_TYPE_DOCK")?;
        self.conn
            .change_property32(PropMode::REPLACE, wid, wm_type, AtomEnum::ATOM, &[dock_type])
            .map_err(|e| PlatformError::X11Error(format!("set window type failed: {}", e)))?;

        // Set always-on-top via _NET_WM_STATE
        let above = self.atom("_NET_WM_STATE_ABOVE")?;
        self.send_wm_state_message(wid, 1, above)?; // _NET_WM_STATE_ADD = 1

        // Skip taskbar
        let skip_taskbar = self.atom("_NET_WM_STATE_SKIP_TASKBAR")?;
        self.send_wm_state_message(wid, 1, skip_taskbar)?;

        self.conn
            .flush()
            .map_err(|e| PlatformError::X11Error(format!("flush failed: {}", e)))?;

        tracing::info!("Set up pet window {} with DOCK type, always-on-top, skip-taskbar", window_id);
        Ok(())
    }

    fn set_click_through(
        &self,
        window_id: u64,
        passthrough_regions: &[Rect],
    ) -> Result<(), PlatformError> {
        use x11rb::protocol::shape;
        use x11rb::protocol::xproto;

        let wid = window_id as u32;

        if passthrough_regions.is_empty() {
            // Empty regions = full passthrough: set an empty input region
            // Create a 0-size pixmap as an empty region
            let pixmap = self
                .conn
                .generate_id()
                .map_err(|e| PlatformError::X11Error(format!("generate_id failed: {}", e)))?;

            self.conn
                .create_pixmap(1, pixmap, wid, 0, 0)
                .map_err(|e| PlatformError::X11Error(format!("create_pixmap failed: {}", e)))?;

            shape::mask(
                &self.conn,
                shape::SO::SET,
                shape::SK::INPUT,
                wid,
                0,
                0,
                pixmap,
            )
            .map_err(|e| PlatformError::X11Error(format!("shape mask failed: {}", e)))?;

            self.conn
                .free_pixmap(pixmap)
                .map_err(|e| PlatformError::X11Error(format!("free_pixmap failed: {}", e)))?;
        } else {
            // Build rectangles from the provided regions
            let rects: Vec<xproto::Rectangle> = passthrough_regions
                .iter()
                .map(|r| xproto::Rectangle {
                    x: r.x as i16,
                    y: r.y as i16,
                    width: r.width as u16,
                    height: r.height as u16,
                })
                .collect();

            shape::rectangles(
                &self.conn,
                shape::SO::SET,
                shape::SK::INPUT,
                ClipOrdering::UNSORTED,
                wid,
                0,
                0,
                &rects,
            )
            .map_err(|e| {
                PlatformError::X11Error(format!("shape rectangles failed: {}", e))
            })?;
        }

        self.conn
            .flush()
            .map_err(|e| PlatformError::X11Error(format!("flush failed: {}", e)))?;

        tracing::debug!(
            "Set click-through on window {} with {} input regions",
            window_id,
            passthrough_regions.len()
        );
        Ok(())
    }

    fn find_terminal_window(&self, project_dir: &str) -> Result<Option<u64>, PlatformError> {
        let windows = self.get_client_list()?;

        for window in windows {
            if !self.is_terminal_window(window) {
                continue;
            }

            if let Some(pid) = self.get_window_pid(window) {
                if let Some(cwd) = Self::read_process_cwd(pid) {
                    if cwd == project_dir || cwd.starts_with(project_dir) {
                        return Ok(Some(window as u64));
                    }
                }
            }
        }

        Ok(None)
    }

    fn focus_window(&self, window_id: u64) -> Result<(), PlatformError> {
        let wid = window_id as u32;
        let active_window = self.atom("_NET_ACTIVE_WINDOW")?;

        let event = ClientMessageEvent::new(
            32,
            wid,
            active_window,
            [1, 0, 0, 0, 0], // source indication = 1 (normal app)
        );

        self.conn
            .send_event(
                false,
                self.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            )
            .map_err(|e| PlatformError::X11Error(format!("send_event failed: {}", e)))?;

        self.conn
            .flush()
            .map_err(|e| PlatformError::X11Error(format!("flush failed: {}", e)))?;

        tracing::debug!("Focused window {}", window_id);
        Ok(())
    }

    fn register_hotkey(&self, key_combo: &str, _action: &str) -> Result<bool, PlatformError> {
        // Parse key combo like "Ctrl+Shift+T"
        let (modifiers, keycode) = parse_key_combo(key_combo)?;

        // Try to grab the key — if it fails, another app has it
        let result = self.conn.grab_key(
            true,
            self.root,
            modifiers,
            keycode,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        );

        match result {
            Ok(cookie) => {
                cookie
                    .check()
                    .map_err(|e| PlatformError::X11Error(format!("grab_key check failed: {}", e)))?;
                self.conn
                    .flush()
                    .map_err(|e| PlatformError::X11Error(format!("flush failed: {}", e)))?;
                tracing::info!("Registered hotkey: {}", key_combo);
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("Failed to grab key {}: {}", key_combo, e);
                Err(PlatformError::HotkeyConflict)
            }
        }
    }
}

/// Parse a key combo string like "Ctrl+Shift+T" into X11 modifier mask and keycode.
/// This is a simplified parser — in production you'd use XKB for proper mapping.
fn parse_key_combo(combo: &str) -> Result<(ModMask, u8), PlatformError> {
    let parts: Vec<&str> = combo.split('+').collect();
    if parts.is_empty() {
        return Err(PlatformError::X11Error(
            "empty key combo".to_string(),
        ));
    }

    let mut modifiers = ModMask::default();
    let mut key_part = None;

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= ModMask::CONTROL,
            "shift" => modifiers |= ModMask::SHIFT,
            "alt" | "mod1" => modifiers |= ModMask::from(u16::from(ModMask::M1)),
            "super" | "mod4" => modifiers |= ModMask::from(u16::from(ModMask::M4)),
            _ => key_part = Some(*part),
        }
    }

    // Map key name to a rough keycode. In practice you'd use XKeysymToKeycode.
    // These are common keycodes for a US keyboard layout.
    let keycode = match key_part {
        Some(k) => match k.to_lowercase().as_str() {
            "a" => 38,
            "b" => 56,
            "c" => 54,
            "d" => 40,
            "e" => 26,
            "f" => 41,
            "g" => 42,
            "h" => 43,
            "i" => 31,
            "j" => 44,
            "k" => 45,
            "l" => 46,
            "m" => 58,
            "n" => 57,
            "o" => 32,
            "p" => 33,
            "q" => 24,
            "r" => 27,
            "s" => 39,
            "t" => 28,
            "u" => 30,
            "v" => 55,
            "w" => 25,
            "x" => 53,
            "y" => 29,
            "z" => 52,
            "space" => 65,
            "return" | "enter" => 36,
            "escape" | "esc" => 9,
            "tab" => 23,
            "f1" => 67,
            "f2" => 68,
            "f3" => 69,
            "f4" => 70,
            "f5" => 71,
            "f6" => 72,
            "f7" => 73,
            "f8" => 74,
            "f9" => 75,
            "f10" => 76,
            "f11" => 95,
            "f12" => 96,
            _ => {
                return Err(PlatformError::X11Error(format!(
                    "unknown key: {}",
                    k
                )));
            }
        },
        None => {
            return Err(PlatformError::X11Error(
                "no key specified in combo".to_string(),
            ));
        }
    };

    Ok((modifiers, keycode))
}
