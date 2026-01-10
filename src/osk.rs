//! On-Screen Keyboard (OSK) integration for TV/living room environments.
//!
//! This module provides a unified interface to show/hide on-screen keyboards
//! across different Linux desktop environments and Wayland compositors.
//!
//! Supported backends (in order of detection priority):
//! - wvkbd: Lightweight OSK for wlroots compositors (Sway, Hyprland, etc.)
//! - GNOME Shell: Built-in OSK via gsettings (accessibility)
//! - Phosh/squeekboard: DBus-based OSK (sm.puri.OSK0)
//! - KDE Plasma: Virtual keyboard via KWin DBus interface

// Allow unused functions - these are part of the public API for future use
#![allow(dead_code)]

use std::env;
use std::path::PathBuf;
use std::process::Command;

use tracing::{debug, info, warn};

/// Detected on-screen keyboard backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OskBackend {
    /// wvkbd - signal-based OSK for wlroots compositors
    Wvkbd,
    /// GNOME Shell built-in OSK via gsettings
    GnomeShell,
    /// Phosh/squeekboard - DBus interface at sm.puri.OSK0
    Squeekboard,
    /// KDE Virtual Keyboard - DBus interface via KWin
    KdePlasma,
}

impl OskBackend {
    /// Human-readable name for the backend.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Wvkbd => "wvkbd",
            Self::GnomeShell => "GNOME Shell OSK",
            Self::Squeekboard => "squeekboard",
            Self::KdePlasma => "KDE Virtual Keyboard",
        }
    }
}

/// Detect available on-screen keyboard backend.
///
/// Returns `None` if no supported OSK is detected.
pub fn detect_backend() -> Option<OskBackend> {
    // Check for wvkbd first (common on wlroots compositors)
    if is_wvkbd_available() {
        debug!("Detected wvkbd OSK backend");
        return Some(OskBackend::Wvkbd);
    }

    // Check for GNOME Shell (standard desktop GNOME)
    if is_gnome_shell_available() {
        debug!("Detected GNOME Shell OSK backend");
        return Some(OskBackend::GnomeShell);
    }

    // Check for Phosh/squeekboard via DBus
    if is_squeekboard_available() {
        debug!("Detected squeekboard OSK backend");
        return Some(OskBackend::Squeekboard);
    }

    // Check for KDE Plasma virtual keyboard
    if is_kde_plasma_available() {
        debug!("Detected KDE Plasma Virtual Keyboard backend");
        return Some(OskBackend::KdePlasma);
    }

    debug!("No on-screen keyboard backend detected");
    None
}

/// Show the on-screen keyboard.
///
/// Returns `true` if the keyboard was successfully shown (or the command was sent).
pub fn show() -> bool {
    let Some(backend) = detect_backend() else {
        warn!("Cannot show OSK: no backend detected");
        return false;
    };

    info!("Showing on-screen keyboard ({})", backend.name());

    match backend {
        OskBackend::Wvkbd => show_wvkbd(),
        OskBackend::GnomeShell => show_gnome_shell(),
        OskBackend::Squeekboard => show_squeekboard(),
        OskBackend::KdePlasma => show_kde_plasma(),
    }
}

/// Hide the on-screen keyboard.
///
/// Returns `true` if the keyboard was successfully hidden (or the command was sent).
pub fn hide() -> bool {
    let Some(backend) = detect_backend() else {
        return false;
    };

    info!("Hiding on-screen keyboard ({})", backend.name());

    match backend {
        OskBackend::Wvkbd => hide_wvkbd(),
        OskBackend::GnomeShell => hide_gnome_shell(),
        OskBackend::Squeekboard => hide_squeekboard(),
        OskBackend::KdePlasma => hide_kde_plasma(),
    }
}

/// Check if an on-screen keyboard is available on this system.
pub fn is_available() -> bool {
    detect_backend().is_some()
}

// =============================================================================
// wvkbd backend (wlroots compositors: Sway, Hyprland, etc.)
// =============================================================================

/// Check if wvkbd is available (running or installed).
fn is_wvkbd_available() -> bool {
    is_wvkbd_running() || command_exists("wvkbd-mobintl") || command_exists("wvkbd-deskintl")
}

/// Check if wvkbd is currently running.
fn is_wvkbd_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "wvkbd-mobintl"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || Command::new("pgrep")
            .args(["-x", "wvkbd-deskintl"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
}

/// Get the PID of running wvkbd process.
fn get_wvkbd_pid() -> Option<i32> {
    for name in ["wvkbd-mobintl", "wvkbd-deskintl"] {
        if let Ok(output) = Command::new("pgrep").args(["-x", name]).output() {
            if output.status.success() {
                let pid_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(pid) = pid_str.trim().parse::<i32>() {
                    return Some(pid);
                }
            }
        }
    }
    None
}

/// Show wvkbd by sending SIGUSR2 signal.
fn show_wvkbd() -> bool {
    if let Some(pid) = get_wvkbd_pid() {
        // SIGUSR2 = show
        Command::new("kill")
            .args(["-USR2", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        // Try to start wvkbd if not running
        start_wvkbd()
    }
}

/// Hide wvkbd by sending SIGUSR1 signal.
fn hide_wvkbd() -> bool {
    if let Some(pid) = get_wvkbd_pid() {
        // SIGUSR1 = hide
        Command::new("kill")
            .args(["-USR1", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        true // Not running, nothing to hide
    }
}

/// Start wvkbd if it's installed but not running.
fn start_wvkbd() -> bool {
    // Try mobintl first, then deskintl
    for binary in ["wvkbd-mobintl", "wvkbd-deskintl"] {
        if command_exists(binary) {
            match Command::new(binary).spawn() {
                Ok(_) => {
                    info!("Started {} on-screen keyboard", binary);
                    return true;
                }
                Err(e) => {
                    warn!("Failed to start {}: {}", binary, e);
                }
            }
        }
    }
    false
}

// =============================================================================
// GNOME Shell backend (gsettings: org.gnome.desktop.a11y.applications)
// =============================================================================

/// Check if GNOME Shell is available.
fn is_gnome_shell_available() -> bool {
    // Check if running under GNOME
    let is_gnome = env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_uppercase().contains("GNOME"))
        .unwrap_or(false);

    if !is_gnome {
        return false;
    }

    // Check if gsettings and the schema are available
    Command::new("gsettings")
        .args([
            "get",
            "org.gnome.desktop.a11y.applications",
            "screen-keyboard-enabled",
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Show GNOME Shell OSK via gsettings.
fn show_gnome_shell() -> bool {
    Command::new("gsettings")
        .args([
            "set",
            "org.gnome.desktop.a11y.applications",
            "screen-keyboard-enabled",
            "true",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Hide GNOME Shell OSK via gsettings.
fn hide_gnome_shell() -> bool {
    Command::new("gsettings")
        .args([
            "set",
            "org.gnome.desktop.a11y.applications",
            "screen-keyboard-enabled",
            "false",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// =============================================================================
// Squeekboard/Phosh backend (DBus: sm.puri.OSK0)
// =============================================================================

/// Check if squeekboard is available via DBus.
fn is_squeekboard_available() -> bool {
    // Check if the DBus service exists
    Command::new("busctl")
        .args(["--user", "introspect", "sm.puri.OSK0", "/sm/puri/OSK0"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Show squeekboard via DBus.
fn show_squeekboard() -> bool {
    Command::new("busctl")
        .args([
            "call",
            "--user",
            "sm.puri.OSK0",
            "/sm/puri/OSK0",
            "sm.puri.OSK0",
            "SetVisible",
            "b",
            "true",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Hide squeekboard via DBus.
fn hide_squeekboard() -> bool {
    Command::new("busctl")
        .args([
            "call",
            "--user",
            "sm.puri.OSK0",
            "/sm/puri/OSK0",
            "sm.puri.OSK0",
            "SetVisible",
            "b",
            "false",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// =============================================================================
// KDE Plasma backend (DBus: org.kde.KWin)
// =============================================================================

/// Check if KDE Plasma Virtual Keyboard is available.
fn is_kde_plasma_available() -> bool {
    // Check if running under KDE
    let is_kde = env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_uppercase().contains("KDE"))
        .unwrap_or(false);

    if !is_kde {
        return false;
    }

    // Check if the DBus interface exists
    Command::new("busctl")
        .args(["--user", "introspect", "org.kde.KWin", "/VirtualKeyboard"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Show KDE Plasma Virtual Keyboard via DBus.
fn show_kde_plasma() -> bool {
    Command::new("busctl")
        .args([
            "call",
            "--user",
            "org.kde.KWin",
            "/VirtualKeyboard",
            "org.kde.kwin.VirtualKeyboard",
            "setEnabled",
            "b",
            "true",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Hide KDE Plasma Virtual Keyboard via DBus.
fn hide_kde_plasma() -> bool {
    Command::new("busctl")
        .args([
            "call",
            "--user",
            "org.kde.KWin",
            "/VirtualKeyboard",
            "org.kde.kwin.VirtualKeyboard",
            "setEnabled",
            "b",
            "false",
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

// =============================================================================
// Utility functions
// =============================================================================

/// Check if a command exists in PATH.
fn command_exists(command: &str) -> bool {
    find_in_path(command).is_some()
}

/// Find a command in PATH.
fn find_in_path(command: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for path in env::split_paths(&path_var) {
        let candidate = path.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_names() {
        assert_eq!(OskBackend::Wvkbd.name(), "wvkbd");
        assert_eq!(OskBackend::GnomeShell.name(), "GNOME Shell OSK");
        assert_eq!(OskBackend::Squeekboard.name(), "squeekboard");
        assert_eq!(OskBackend::KdePlasma.name(), "KDE Virtual Keyboard");
    }

    #[test]
    fn test_detect_backend_does_not_panic() {
        // Just ensure detection doesn't panic
        let _ = detect_backend();
    }

    #[test]
    fn test_is_available_does_not_panic() {
        let _ = is_available();
    }
}
