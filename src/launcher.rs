use std::process::{Command, Stdio};
use thiserror::Error;
use tracing::{error, info};

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
