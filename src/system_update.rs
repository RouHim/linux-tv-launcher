use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use thiserror::Error;
use tracing::info;

#[derive(Debug)]
struct UpdateCommand {
    program: String,
    args: Vec<String>,
    label: &'static str,
}

impl UpdateCommand {
    fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("No supported system updater found. Install pkexec or sudo and a supported package manager.")]
    NoSupportedUpdater,
    #[error("Failed to launch system update command `{command}`: {source}")]
    LaunchFailed {
        command: String,
        source: std::io::Error,
    },
}

pub fn run_update() -> Result<String, UpdateError> {
    let command = detect_update_command().ok_or(UpdateError::NoSupportedUpdater)?;

    info!("Starting system update: {}", command.display());

    Command::new(&command.program)
        .args(&command.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| UpdateError::LaunchFailed {
            command: command.display(),
            source: err,
        })?;

    Ok(format!("System update started ({})", command.label))
}

fn detect_update_command() -> Option<UpdateCommand> {
    let mode = privilege_mode();
    if matches!(mode, PrivilegeMode::None) && !is_root_user() {
        return None;
    }

    if command_exists("apt-get") {
        return Some(build_shell_command(
            mode,
            "apt-get update && apt-get upgrade -y",
            "apt-get",
        ));
    }

    if command_exists("dnf") {
        return Some(build_command(mode, "dnf", &["upgrade", "-y"], "dnf"));
    }

    if command_exists("pacman") {
        return Some(build_command(
            mode,
            "pacman",
            &["-Syu", "--noconfirm"],
            "pacman",
        ));
    }

    if command_exists("zypper") {
        return Some(build_command(mode, "zypper", &["update", "-y"], "zypper"));
    }

    None
}

#[derive(Clone, Copy)]
enum PrivilegeMode {
    Pkexec,
    Sudo,
    None,
}

fn privilege_mode() -> PrivilegeMode {
    if command_exists("pkexec") {
        PrivilegeMode::Pkexec
    } else if command_exists("sudo") {
        PrivilegeMode::Sudo
    } else {
        PrivilegeMode::None
    }
}

fn build_command_with_privilege(
    mode: PrivilegeMode,
    program: &str,
    mut args: Vec<String>,
    label: &'static str,
) -> UpdateCommand {
    match mode {
        PrivilegeMode::Pkexec => {
            args.insert(0, program.to_string());
            UpdateCommand {
                program: "pkexec".to_string(),
                args,
                label,
            }
        }
        PrivilegeMode::Sudo => {
            args.insert(0, program.to_string());
            args.insert(0, "-n".to_string());
            UpdateCommand {
                program: "sudo".to_string(),
                args,
                label,
            }
        }
        PrivilegeMode::None => UpdateCommand {
            program: program.to_string(),
            args,
            label,
        },
    }
}

fn build_command(
    mode: PrivilegeMode,
    program: &str,
    args: &[&str],
    label: &'static str,
) -> UpdateCommand {
    let args = args.iter().map(|arg| (*arg).to_string()).collect();
    build_command_with_privilege(mode, program, args, label)
}

fn build_shell_command(mode: PrivilegeMode, command: &str, label: &'static str) -> UpdateCommand {
    build_command_with_privilege(
        mode,
        "sh",
        vec!["-c".to_string(), command.to_string()],
        label,
    )
}

fn command_exists(command: &str) -> bool {
    find_in_path(command).is_some()
}

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

fn is_root_user() -> bool {
    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        return false;
    };

    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("Uid:") {
            let mut parts = rest.split_whitespace();
            return matches!(parts.next(), Some("0"));
        }
    }

    false
}
