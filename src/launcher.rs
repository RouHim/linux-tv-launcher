use std::process::{Command, Stdio};
use thiserror::Error;
use tracing::{error, info};
use urlencoding::decode;

use crate::focus_manager::MonitorTarget;

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
    info!("Launching: {}", exec);

    // Split the exec string into command and args
    // Be careful with quotes, but for now simple split
    let parts: Vec<&str> = exec.split_whitespace().collect();
    if parts.is_empty() {
        return Err(LaunchError::EmptyCommand);
    }

    let cmd = parts[0];
    let args = &parts[1..];

    match Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => {
            let pid = child.id();
            info!("Successfully launched {} (PID: {})", cmd, pid);
            Ok(pid)
        }
        Err(e) => {
            error!("Failed to launch {}: {}", cmd, e);
            Err(LaunchError::LaunchFailed {
                command: cmd.to_string(),
                source: e,
            })
        }
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
            info!("Detected Heroic launch for app: {}", name);

            let mut targets = vec![
                MonitorTarget::EnvVarEq("LEGENDARY_GAME_ID".to_string(), name.clone()),
                MonitorTarget::EnvVarEq("HeroicAppName".to_string(), name.clone()),
                MonitorTarget::CmdLineContains(item_name.to_string()),
            ];

            // Add exact executable match if available
            if let Some(exe) = game_executable {
                info!("Monitoring executable for {}: {}", name, exe);
                targets.push(MonitorTarget::CmdLineContains(exe.clone()));
            }

            let sanitized_name = item_name.replace(":", "");
            if sanitized_name != item_name {
                targets.push(MonitorTarget::CmdLineContains(sanitized_name));
            }

            return Some(MonitorTarget::Any(targets));
        }
    }

    None
}
