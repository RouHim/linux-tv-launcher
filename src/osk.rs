//! On-Screen Keyboard (OSK) integration.
//!
//! Provides a unified interface to show/hide on-screen keyboards
//! across different Linux desktop environments and Wayland compositors.

use std::env;
use std::process::Command;
use tracing::{debug, info, warn};

/// Manager for the On-Screen Keyboard.
///
/// Handles detection of the available OSK backend and manages its state.
pub struct OskManager {
    backend: Option<Box<dyn OskBackend>>,
    /// Tracks if the OSK was enabled by this application (used for restoration on exit).
    enabled_by_app: bool,
}

#[allow(dead_code)]
impl OskManager {
    /// Create a new OSK Manager and detect the available backend.
    pub fn new() -> Self {
        let backend = detect_backend();
        Self {
            backend,
            enabled_by_app: false,
        }
    }

    /// Show the on-screen keyboard.
    pub fn show(&mut self) {
        let Some(backend) = &self.backend else {
            warn!("Cannot show OSK: no backend detected");
            return;
        };

        info!("Showing on-screen keyboard ({})", backend.name());

        // Check if we need to track state (currently primarily for GNOME)
        // If the OSK is currently disabled, mark that we are enabling it.
        if !self.enabled_by_app {
            if let Ok(false) = backend.is_enabled() {
                self.enabled_by_app = true;
            }
        }

        if let Err(e) = backend.show() {
            warn!("Failed to show OSK: {}", e);
        }
    }

    /// Hide the on-screen keyboard.
    pub fn hide(&self) {
        let Some(backend) = &self.backend else {
            return;
        };

        info!("Hiding on-screen keyboard ({})", backend.name());
        if let Err(e) = backend.hide() {
            warn!("Failed to hide OSK: {}", e);
        }
    }

    /// Restore the OSK state if it was modified by the application.
    ///
    /// This should be called on application exit.
    pub fn restore(&mut self) {
        if self.enabled_by_app {
            info!("Restoring OSK state (disabling on exit)");
            if let Some(backend) = &self.backend {
                if let Err(e) = backend.hide() {
                    warn!("Failed to restore OSK state: {}", e);
                }
            }
            self.enabled_by_app = false;
        }
    }

    /// Check if an OSK backend is available.
    pub fn is_available(&self) -> bool {
        self.backend.is_some()
    }
}

/// Trait representing an On-Screen Keyboard backend.
trait OskBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn show(&self) -> Result<(), String>;
    fn hide(&self) -> Result<(), String>;
    /// Check if the OSK is currently enabled/visible.
    /// Returns error if check is not supported/failed.
    fn is_enabled(&self) -> Result<bool, String>;
}

fn detect_backend() -> Option<Box<dyn OskBackend>> {
    // 1. wvkbd
    if WvkbdBackend::is_available() {
        debug!("Detected wvkbd OSK backend");
        return Some(Box::new(WvkbdBackend));
    }

    // 2. GNOME Shell
    if GnomeBackend::is_available() {
        debug!("Detected GNOME Shell OSK backend");
        return Some(Box::new(GnomeBackend));
    }

    // 3. Squeekboard
    if SqueekboardBackend::is_available() {
        debug!("Detected squeekboard OSK backend");
        return Some(Box::new(SqueekboardBackend));
    }

    // 4. KDE Plasma
    if KdeBackend::is_available() {
        debug!("Detected KDE Plasma Virtual Keyboard backend");
        return Some(Box::new(KdeBackend));
    }

    debug!("No on-screen keyboard backend detected");
    None
}

// =============================================================================
// wvkbd backend
// =============================================================================

struct WvkbdBackend;

impl WvkbdBackend {
    fn is_available() -> bool {
        Self::is_running() || command_exists("wvkbd-mobintl") || command_exists("wvkbd-deskintl")
    }

    fn is_running() -> bool {
        check_pgrep("wvkbd-mobintl") || check_pgrep("wvkbd-deskintl")
    }

    fn get_pid() -> Option<i32> {
        get_pid_by_name("wvkbd-mobintl").or_else(|| get_pid_by_name("wvkbd-deskintl"))
    }
}

impl OskBackend for WvkbdBackend {
    fn name(&self) -> &'static str {
        "wvkbd"
    }

    fn show(&self) -> Result<(), String> {
        if let Some(pid) = Self::get_pid() {
            // SIGUSR2 = show
            run_command("kill", &["-USR2", &pid.to_string()])
        } else {
            // Start it
            for binary in ["wvkbd-mobintl", "wvkbd-deskintl"] {
                if command_exists(binary) {
                    Command::new(binary).spawn().map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
            Err("wvkbd binary not found".to_string())
        }
    }

    fn hide(&self) -> Result<(), String> {
        if let Some(pid) = Self::get_pid() {
            // SIGUSR1 = hide
            run_command("kill", &["-USR1", &pid.to_string()])
        } else {
            Ok(())
        }
    }

    fn is_enabled(&self) -> Result<bool, String> {
        Err("Not supported".to_string())
    }
}

// =============================================================================
// GNOME Backend
// =============================================================================

struct GnomeBackend;

impl GnomeBackend {
    fn is_available() -> bool {
        is_gnome_session()
            && Command::new("gsettings")
                .args([
                    "get",
                    "org.gnome.desktop.a11y.applications",
                    "screen-keyboard-enabled",
                ])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }
}

impl OskBackend for GnomeBackend {
    fn name(&self) -> &'static str {
        "GNOME Shell OSK"
    }

    fn show(&self) -> Result<(), String> {
        run_command(
            "gsettings",
            &[
                "set",
                "org.gnome.desktop.a11y.applications",
                "screen-keyboard-enabled",
                "true",
            ],
        )
    }

    fn hide(&self) -> Result<(), String> {
        run_command(
            "gsettings",
            &[
                "set",
                "org.gnome.desktop.a11y.applications",
                "screen-keyboard-enabled",
                "false",
            ],
        )
    }

    fn is_enabled(&self) -> Result<bool, String> {
        let output = Command::new("gsettings")
            .args([
                "get",
                "org.gnome.desktop.a11y.applications",
                "screen-keyboard-enabled",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err("gsettings failed".to_string());
        }

        let value = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_lowercase();
        match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(format!("Unknown value: {}", value)),
        }
    }
}

fn is_gnome_session() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_uppercase().contains("GNOME"))
        .unwrap_or(false)
}

// =============================================================================
// Squeekboard Backend
// =============================================================================

struct SqueekboardBackend;

impl SqueekboardBackend {
    fn is_available() -> bool {
        check_dbus_service("sm.puri.OSK0", "/sm/puri/OSK0")
    }
}

impl OskBackend for SqueekboardBackend {
    fn name(&self) -> &'static str {
        "squeekboard"
    }

    fn show(&self) -> Result<(), String> {
        run_dbus_call(
            "sm.puri.OSK0",
            "/sm/puri/OSK0",
            "sm.puri.OSK0",
            "SetVisible",
            "b",
            "true",
        )
    }

    fn hide(&self) -> Result<(), String> {
        run_dbus_call(
            "sm.puri.OSK0",
            "/sm/puri/OSK0",
            "sm.puri.OSK0",
            "SetVisible",
            "b",
            "false",
        )
    }

    fn is_enabled(&self) -> Result<bool, String> {
        // Squeekboard DBus property could be checked, but omitting for now
        Err("Not supported".to_string())
    }
}

// =============================================================================
// KDE Plasma Backend
// =============================================================================

struct KdeBackend;

impl KdeBackend {
    fn is_available() -> bool {
        is_kde_session() && check_dbus_service("org.kde.KWin", "/VirtualKeyboard")
    }
}

impl OskBackend for KdeBackend {
    fn name(&self) -> &'static str {
        "KDE Virtual Keyboard"
    }

    fn show(&self) -> Result<(), String> {
        run_dbus_call(
            "org.kde.KWin",
            "/VirtualKeyboard",
            "org.kde.kwin.VirtualKeyboard",
            "setEnabled",
            "b",
            "true",
        )
    }

    fn hide(&self) -> Result<(), String> {
        run_dbus_call(
            "org.kde.KWin",
            "/VirtualKeyboard",
            "org.kde.kwin.VirtualKeyboard",
            "setEnabled",
            "b",
            "false",
        )
    }

    fn is_enabled(&self) -> Result<bool, String> {
        Err("Not supported".to_string())
    }
}

fn is_kde_session() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_uppercase().contains("KDE"))
        .unwrap_or(false)
}

// =============================================================================
// Helpers
// =============================================================================

fn command_exists(command: &str) -> bool {
    if let Some(path_var) = env::var_os("PATH") {
        for path in env::split_paths(&path_var) {
            if path.join(command).is_file() {
                return true;
            }
        }
    }
    false
}

fn check_pgrep(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn get_pid_by_name(name: &str) -> Option<i32> {
    if let Ok(output) = Command::new("pgrep").args(["-x", name]).output() {
        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout);
            return pid_str.trim().parse::<i32>().ok();
        }
    }
    None
}

fn run_command(cmd: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Command {} failed", cmd))
    }
}

fn check_dbus_service(dest: &str, path: &str) -> bool {
    Command::new("busctl")
        .args(["--user", "introspect", dest, path])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_dbus_call(
    dest: &str,
    path: &str,
    interface: &str,
    method: &str,
    type_sig: &str,
    value: &str,
) -> Result<(), String> {
    run_command(
        "busctl",
        &[
            "call", "--user", dest, path, interface, method, type_sig, value,
        ],
    )
}
