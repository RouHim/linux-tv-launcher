use std::path::Path;
use std::process::{Command, Stdio};
use thiserror::Error;
use urlencoding::decode;

use crate::focus_manager::MonitorTarget;

/// Desktop entry field codes that should be stripped from exec commands
/// See: https://specifications.freedesktop.org/desktop-entry-spec/latest/exec-variables.html
const DESKTOP_FIELD_CODES: &[&str] = &[
    "%f", "%F", "%u", "%U", "%d", "%D", "%n", "%N", "%i", "%c", "%k", "%v", "%m",
];

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("No command specified to launch.")]
    EmptyCommand,
    #[error("Failed to launch `{command}`: {source}")]
    LaunchFailed {
        command: String,
        source: std::io::Error,
    },
}

pub fn launch_app(exec: &str) -> Result<u32, LaunchError> {
    if exec.trim().is_empty() {
        return Err(LaunchError::EmptyCommand);
    }

    // Use sh -c to handle complex command strings with quotes/args properly
    match Command::new("sh")
        .arg("-c")
        .arg(exec)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => {
            let pid = child.id();
            Ok(pid)
        }
        Err(e) => Err(LaunchError::LaunchFailed {
            command: exec.to_string(),
            source: e,
        }),
    }
}

pub fn resolve_monitor_target(
    exec: &str,
    item_name: &str,
    game_executable: Option<&String>,
) -> Option<MonitorTarget> {
    // Check if it's a Steam game launch
    let steam_launch_prefix = "steam -applaunch ";
    let heroic_launch_prefix = "xdg-open heroic://launch/";

    if exec.starts_with(steam_launch_prefix) {
        let appid = exec
            .trim_start_matches(steam_launch_prefix)
            .trim()
            .to_string();
        // We still launch the steam command, but we monitor the AppId
        return Some(MonitorTarget::SteamAppId(appid));
    }

    if exec.starts_with(heroic_launch_prefix) {
        let url_part = exec.trim_start_matches(heroic_launch_prefix).trim();
        let parts: Vec<&str> = url_part.split('/').collect();

        let mut app_name = None;

        if parts.len() >= 2 {
            // store/app_name
            if let Ok(decoded) = decode(parts[1]) {
                app_name = Some(decoded.to_string());
            }
        } else if parts.len() == 1 {
            // app_name
            if let Ok(decoded) = decode(parts[0]) {
                app_name = Some(decoded.to_string());
            }
        }

        if let Some(name) = app_name {
            let mut targets = vec![
                MonitorTarget::EnvVarEq("LEGENDARY_GAME_ID".to_string(), name.clone()),
                MonitorTarget::EnvVarEq("HeroicAppName".to_string(), name.clone()),
                MonitorTarget::CmdLineContains(item_name.to_string()),
            ];

            // Add exact executable match if available
            if let Some(exe) = game_executable {
                targets.push(MonitorTarget::CmdLineContains(exe.clone()));
            }

            let sanitized_name = item_name.replace(":", "");
            if sanitized_name != item_name {
                targets.push(MonitorTarget::CmdLineContains(sanitized_name));
            }

            return Some(MonitorTarget::Any(targets));
        }
    }

    // For regular applications, use command-line pattern matching
    Some(create_app_monitor_target(exec, item_name))
}

/// Extracts tokens from a shell-like command line, respecting quotes.
fn split_exec_tokens(exec: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;
    let mut had_quotes = false;

    for ch in exec.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }

        if ch == '\\' && !in_single {
            escape = true;
            continue;
        }

        if ch == '\'' && !in_double {
            in_single = !in_single;
            had_quotes = true;
            continue;
        }

        if ch == '"' && !in_single {
            in_double = !in_double;
            had_quotes = true;
            continue;
        }

        if ch.is_whitespace() && !in_single && !in_double {
            if !current.is_empty() || had_quotes {
                tokens.push(std::mem::take(&mut current));
                had_quotes = false;
            }
            continue;
        }

        current.push(ch);
    }

    if escape {
        current.push('\\');
    }

    if !current.is_empty() || had_quotes {
        tokens.push(current);
    }

    tokens
}

/// Extracts the executable name from an exec command string.
///
/// Handles paths, arguments, desktop field codes (`%f`, `%U`, etc.),
/// and `env VAR=val` prefixes.
fn extract_executable_name(exec: &str) -> Option<String> {
    let is_skippable = |part: &str| {
        part.is_empty()
            || part == "env"
            || (part.contains('=') && !part.starts_with('-'))
            || DESKTOP_FIELD_CODES.contains(&part)
    };

    let cmd = split_exec_tokens(exec)
        .into_iter()
        .find(|part| !is_skippable(part))?;

    Path::new(&cmd)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Creates a monitor target for a regular application launch.
///
/// Combines executable name and item name patterns for robust process detection.
fn create_app_monitor_target(exec: &str, item_name: &str) -> MonitorTarget {
    let exe_name = extract_executable_name(exec);

    // Check if exe_name and item_name are equivalent (case-insensitive)
    let names_match = exe_name
        .as_ref()
        .is_some_and(|e| e.eq_ignore_ascii_case(item_name));

    match (exe_name, names_match) {
        (Some(exe), true) => MonitorTarget::CmdLineContains(exe),
        (Some(exe), false) if !item_name.is_empty() => MonitorTarget::Any(vec![
            MonitorTarget::CmdLineContains(exe),
            MonitorTarget::CmdLineContains(item_name.to_string()),
        ]),
        (Some(exe), false) => MonitorTarget::CmdLineContains(exe),
        (None, _) => MonitorTarget::CmdLineContains(item_name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_executable_name() {
        // Simple command
        assert_eq!(extract_executable_name("firefox"), Some("firefox".into()));

        // With path
        assert_eq!(
            extract_executable_name("/usr/bin/firefox"),
            Some("firefox".into())
        );

        // With arguments
        assert_eq!(
            extract_executable_name("firefox --new-window"),
            Some("firefox".into())
        );

        // With field codes
        assert_eq!(
            extract_executable_name("firefox %u"),
            Some("firefox".into())
        );
        assert_eq!(
            extract_executable_name("nautilus %U"),
            Some("nautilus".into())
        );

        // With env prefix
        assert_eq!(
            extract_executable_name("env LANG=C firefox"),
            Some("firefox".into())
        );
        assert_eq!(
            extract_executable_name("env VAR1=a VAR2=b /usr/bin/app"),
            Some("app".into())
        );

        // Quoted or escaped paths
        assert_eq!(
            extract_executable_name("\"/opt/My Game/game\" --flag"),
            Some("game".into())
        );
        assert_eq!(
            extract_executable_name("/opt/My\\ Game/game --flag"),
            Some("game".into())
        );
        assert_eq!(
            extract_executable_name("env VAR=1 \"/opt/My Game/game\" --flag"),
            Some("game".into())
        );

        // Empty
        assert_eq!(extract_executable_name(""), None);
    }

    #[test]
    fn test_create_app_monitor_target() {
        // Different names -> multiple targets
        let target = create_app_monitor_target("firefox", "Mozilla Firefox");
        assert!(matches!(target, MonitorTarget::Any(t) if t.len() == 2));

        // Same name (case-insensitive) -> single target
        let target = create_app_monitor_target("firefox", "Firefox");
        assert!(matches!(target, MonitorTarget::CmdLineContains(s) if s == "firefox"));

        // Exact same name -> single target
        let target = create_app_monitor_target("firefox", "firefox");
        assert!(matches!(target, MonitorTarget::CmdLineContains(s) if s == "firefox"));
    }

    #[test]
    fn test_launch_app_handles_quoted_arguments() {
        use std::fs;
        use uuid::Uuid;

        let temp_dir = std::env::temp_dir().join(format!("launcher_test_{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        // Command: touch "filename with spaces"
        // If quotes are respected, it creates "filename with spaces"
        // If not, it creates "filename" and "with" and "spaces" (or fails differently)
        let file_name = "filename with spaces";
        let file_path = temp_dir.join(file_name);

        // We use absolute path to ensure touch works everywhere
        // But simply "touch" should be in PATH
        let exec = format!("touch \"{}\"", file_path.to_string_lossy());

        let res = launch_app(&exec);
        assert!(res.is_ok());

        // Give it a moment to execute
        std::thread::sleep(std::time::Duration::from_millis(200));

        assert!(file_path.exists(), "File with spaces should exist");

        // Verify no split files were created
        let split_part = temp_dir.join("filename");
        assert!(!split_part.exists(), "Should not create split filename");

        let _ = fs::remove_dir_all(temp_dir);
    }
}
