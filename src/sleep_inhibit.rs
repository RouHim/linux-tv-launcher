//! Sleep inhibition manager for preventing system sleep during launcher activity.
//!
//! Uses DBus interfaces with a fallback chain:
//! 1. PRIMARY: `org.freedesktop.login1.Manager.Inhibit` (FD-based, crash-safe)
//! 2. FALLBACK: `org.freedesktop.portal.Inhibit` (XDG Portal)

use tracing::{error, info, warn};
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedFd, OwnedObjectPath, Value};

/// Represents the active inhibition handle.
enum InhibitorHandle {
    /// File descriptor from logind - auto-releases on drop (crash-safe).
    LogindFd(OwnedFd),
    /// Object path from portal - requires explicit Close call.
    PortalHandle {
        connection: Connection,
        handle: OwnedObjectPath,
    },
}

/// Manager for preventing system sleep while the launcher is active.
///
/// Follows the manager pattern: struct with `new()`, state fields, cleanup method.
/// Implements `Drop` to ensure cleanup on panic/exit.
pub struct SleepInhibitor {
    /// Active inhibition handle, if any.
    inhibitor: Option<InhibitorHandle>,
}

impl SleepInhibitor {
    /// Create a new sleep inhibitor manager with no active inhibition.
    ///
    /// This is fast and non-blocking - no DBus calls are made here.
    /// Call `acquire()` to actually inhibit sleep.
    pub fn new() -> Self {
        Self { inhibitor: None }
    }

    /// Attempt to acquire sleep inhibition.
    ///
    /// Tries logind first (FD-based, crash-safe), then falls back to portal.
    /// Logs success/failure but never panics.
    pub fn acquire(&mut self) {
        // Already have an active inhibition
        if self.inhibitor.is_some() {
            return;
        }

        // Try logind first (primary)
        match Self::try_logind_inhibit() {
            Ok(fd) => {
                info!("Sleep inhibition acquired via logind");
                self.inhibitor = Some(InhibitorHandle::LogindFd(fd));
                return;
            }
            Err(e) => {
                warn!("logind inhibit failed, trying portal fallback: {}", e);
            }
        }

        // Try portal fallback
        match Self::try_portal_inhibit() {
            Ok((connection, handle)) => {
                info!("Sleep inhibition acquired via portal (fallback)");
                self.inhibitor = Some(InhibitorHandle::PortalHandle { connection, handle });
            }
            Err(e) => {
                error!("All sleep inhibition methods failed: {}", e);
            }
        }
    }

    /// Explicitly release the sleep inhibition.
    ///
    /// Safe to call multiple times or when no inhibition is active.
    pub fn release(&mut self) {
        if let Some(handle) = self.inhibitor.take() {
            match handle {
                InhibitorHandle::LogindFd(_fd) => {
                    // FD auto-releases on drop - nothing to do
                    info!("Sleep inhibition released (logind FD dropped)");
                }
                InhibitorHandle::PortalHandle { connection, handle } => {
                    // Portal requires explicit Close call
                    if let Err(e) = Self::close_portal_inhibit(&connection, &handle) {
                        warn!("Failed to close portal inhibit handle: {}", e);
                    } else {
                        info!("Sleep inhibition released (portal)");
                    }
                }
            }
        }
    }

    /// Try to acquire inhibition via systemd-logind.
    ///
    /// Returns an owned file descriptor that keeps the inhibition active.
    /// The inhibition is automatically released when the FD is closed (crash-safe).
    fn try_logind_inhibit() -> Result<OwnedFd, zbus::Error> {
        let conn = Connection::system()?;

        let reply = conn.call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "Inhibit",
            &("sleep", "rhinco-tv", "Launcher active", "block"),
        )?;

        let fd: OwnedFd = reply.body().deserialize()?;
        Ok(fd)
    }

    /// Try to acquire inhibition via XDG Portal.
    ///
    /// Returns the connection and object path handle.
    /// The inhibition must be explicitly released via Close().
    fn try_portal_inhibit() -> Result<(Connection, OwnedObjectPath), zbus::Error> {
        let conn = Connection::session()?;

        // Flags: 4 (Suspend) | 8 (Idle) = 12
        let flags: u32 = 12;

        // Empty options dict
        let options: std::collections::HashMap<&str, Value<'_>> = std::collections::HashMap::new();

        let reply = conn.call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.Inhibit"),
            "Inhibit",
            &("", flags, options),
        )?;

        let handle: OwnedObjectPath = reply.body().deserialize()?;
        Ok((conn, handle))
    }

    /// Close a portal inhibit handle.
    fn close_portal_inhibit(
        connection: &Connection,
        handle: &OwnedObjectPath,
    ) -> Result<(), zbus::Error> {
        connection.call_method(
            Some("org.freedesktop.portal.Desktop"),
            handle.as_str(),
            Some("org.freedesktop.portal.Request"),
            "Close",
            &(),
        )?;
        Ok(())
    }
}

impl Default for SleepInhibitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SleepInhibitor {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_inactive_inhibitor() {
        let _inhibitor = SleepInhibitor::new();
    }

    #[test]
    fn test_default_creates_inactive_inhibitor() {
        let _inhibitor = SleepInhibitor::default();
    }

    #[test]
    fn test_release_on_inactive_is_safe() {
        let mut inhibitor = SleepInhibitor::new();
        // Should not panic
        inhibitor.release();
    }

    #[test]
    fn test_multiple_release_calls_are_safe() {
        let mut inhibitor = SleepInhibitor::new();
        inhibitor.release();
        inhibitor.release();
        inhibitor.release();
    }

    #[test]
    fn test_inhibitor_handle_enum_variants() {
        // Test that the enum variants are properly defined
        // This is a compile-time check more than runtime
        fn _assert_send<T: Send>() {}
        fn _assert_sync<T: Sync>() {}

        // SleepInhibitor should be Send (can be moved between threads)
        _assert_send::<SleepInhibitor>();
    }

    /// Test that logind inhibit returns appropriate error when DBus unavailable.
    /// This test will fail gracefully in CI environments without DBus.
    #[test]
    fn test_logind_inhibit_handles_missing_dbus() {
        // This should return an error, not panic
        let result = SleepInhibitor::try_logind_inhibit();
        // We don't assert success/failure since it depends on the environment
        // The important thing is it doesn't panic
        drop(result);
    }

    /// Test that portal inhibit returns appropriate error when DBus unavailable.
    #[test]
    fn test_portal_inhibit_handles_missing_dbus() {
        // This should return an error, not panic
        let result = SleepInhibitor::try_portal_inhibit();
        // We don't assert success/failure since it depends on the environment
        drop(result);
    }

    /// Test the full acquire flow handles errors gracefully.
    #[test]
    fn test_acquire_handles_all_failures_gracefully() {
        let mut inhibitor = SleepInhibitor::new();
        // In CI without DBus, this should fail gracefully
        inhibitor.acquire();
        // Whether it succeeded or failed, we should be able to release
        inhibitor.release();
    }
}
