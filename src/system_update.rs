use crate::sudo_askpass::{get_askpass_script_path, get_socket_path};
use crate::system_update_state::{SystemUpdateProgress, UpdateStatus};
use iced::futures::{SinkExt, Stream};
use std::collections::HashMap;
use std::env;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

type UpdateCommand = (String, Vec<String>, HashMap<String, String>);

pub fn system_update_stream() -> impl Stream<Item = SystemUpdateProgress> {
    iced::stream::channel(
        100,
        |mut output: iced::futures::channel::mpsc::Sender<SystemUpdateProgress>| async move {
            tracing::info!("System update stream started");
            send_status(&mut output, UpdateStatus::Starting).await;

            let (program, args, env_vars) = match get_update_command() {
                Ok(command) => command,
                Err(message) => {
                    send_failed(&mut output, message).await;
                    return;
                }
            };

            tracing::info!(program = %program, args = ?args, "Spawning update command");
            let mut cmd = Command::new(&program);
            cmd.args(&args);
            cmd.envs(env_vars);

            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            cmd.stdin(Stdio::null()); // Ensure we don't hang if the process asks for input
            cmd.kill_on_drop(true);

            let mut updated_packages: Vec<String> = Vec::new();

            match cmd.spawn() {
                Ok(child) => {
                    monitor_child(child, &mut output, &mut updated_packages).await;
                }
                Err(e) => {
                    let msg = format!("Failed to spawn update process: {}", e);
                    send_failed(&mut output, msg).await;
                }
            }
        },
    )
}

async fn monitor_child(
    mut child: tokio::process::Child,
    output: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    updated_packages: &mut Vec<String>,
) {
    let mut stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            send_failed(output, "Failed to capture stdout".to_string()).await;
            return;
        }
    };
    let mut stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            send_failed(output, "Failed to capture stderr".to_string()).await;
            return;
        }
    };

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();
    let mut read_buf_stdout = [0u8; 1024];
    let mut read_buf_stderr = [0u8; 1024];
    let mut stdout_done = false;
    let mut stderr_done = false;
    let mut child_status: Option<Result<std::process::ExitStatus, std::io::Error>> = None;

    loop {
        tokio::select! {
            res = stdout.read(&mut read_buf_stdout), if !stdout_done => {
                match res {
                    Ok(0) => {
                        flush_output_buffer(&mut stdout_buf, output, updated_packages).await;
                        stdout_done = true;
                    }
                    Ok(n) => {
                        stdout_buf.extend_from_slice(&read_buf_stdout[..n]);
                        process_output_buffer(&mut stdout_buf, output, updated_packages).await;
                    }
                    Err(e) => {
                        send_failed(output, format!("Error reading stdout: {}", e)).await;
                        return;
                    }
                }
            }
            res = stderr.read(&mut read_buf_stderr), if !stderr_done => {
                match res {
                    Ok(0) => {
                        flush_output_buffer(&mut stderr_buf, output, updated_packages).await;
                        stderr_done = true;
                    }
                    Ok(n) => {
                        stderr_buf.extend_from_slice(&read_buf_stderr[..n]);
                        process_output_buffer(&mut stderr_buf, output, updated_packages).await;
                    }
                    Err(e) => {
                        send_failed(output, format!("Error reading stderr: {}", e)).await;
                        return;
                    }
                }
            }
            status = child.wait(), if child_status.is_none() => {
                child_status = Some(status);
            }
        }

        if stdout_done && stderr_done {
            let status = match child_status.take() {
                Some(status) => status,
                None => child.wait().await,
            };
            handle_child_exit(status, output, updated_packages).await;
            return;
        }
    }
}

async fn send_status(
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    status: UpdateStatus,
) {
    let _ = sender
        .send(SystemUpdateProgress::StatusChange(status))
        .await;
}

async fn send_failed(
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    message: String,
) {
    let _ = sender
        .send(SystemUpdateProgress::StatusChange(UpdateStatus::Failed(
            message,
        )))
        .await;
}

async fn handle_child_exit(
    status: Result<std::process::ExitStatus, std::io::Error>,
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    updated_packages: &[String],
) {
    match status {
        Ok(status) => {
            if status.success() {
                let restart_required = check_restart_required(updated_packages);
                send_status(sender, UpdateStatus::Completed { restart_required }).await;
            } else {
                let msg = format!("Process exited with code: {:?}", status.code());
                send_failed(sender, msg).await;
            }
        }
        Err(e) => {
            let msg = format!("Process wait failed: {}", e);
            send_failed(sender, msg).await;
        }
    }
}

// Processes the buffer: extracts complete lines, parses them.
async fn process_output_buffer(
    buffer: &mut Vec<u8>,
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    updated_packages: &mut Vec<String>,
) {
    loop {
        let n_pos = buffer.iter().position(|&b| b == b'\n');
        let r_pos = buffer.iter().position(|&b| b == b'\r');

        let pos = match (n_pos, r_pos) {
            (Some(n), Some(r)) => Some(std::cmp::min(n, r)),
            (Some(n), None) => Some(n),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        };

        if let Some(p) = pos {
            let line_bytes: Vec<u8> = buffer.drain(..=p).collect();
            let line = String::from_utf8_lossy(&line_bytes);
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                parse_output_line(trimmed, sender, updated_packages).await;
            }
        } else {
            break;
        }
    }
}

async fn flush_output_buffer(
    buffer: &mut Vec<u8>,
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    updated_packages: &mut Vec<String>,
) {
    if buffer.is_empty() {
        return;
    }

    let remaining = String::from_utf8_lossy(buffer).to_string();
    let trimmed = remaining.trim();
    if trimmed.is_empty() {
        buffer.clear();
        return;
    }

    parse_output_line(trimmed, sender, updated_packages).await;
    buffer.clear();
}

async fn parse_output_line(
    line: &str,
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    updated_packages: &mut Vec<String>,
) {
    // Log output to application logger

    let lower = line.to_lowercase();
    let _ = sender
        .send(SystemUpdateProgress::LogLine(line.to_string()))
        .await;

    // Detect explicit build errors
    if lower.starts_with("-> error making:") {
        let msg = line
            .trim_start_matches("-> error making:")
            .trim()
            .to_string();
        let _ = sender
            .send(SystemUpdateProgress::StatusChange(UpdateStatus::Failed(
                msg,
            )))
            .await;
        return;
    }

    let new_status = if lower.contains("synchronizing package databases") {
        Some(UpdateStatus::SyncingDatabases)
    } else if lower.contains("starting full system upgrade") {
        Some(UpdateStatus::CheckingUpdates)
    } else if let Some(pkg) = parse_building_package(line) {
        Some(UpdateStatus::Building { package: pkg })
    } else if let Some(pkg) = parse_downloading_package(line) {
        Some(UpdateStatus::Downloading { package: Some(pkg) })
    } else if lower.contains("downloading") {
        Some(UpdateStatus::Downloading { package: None })
    } else if let Some((current, total, pkg)) = parse_install_progress(line) {
        updated_packages.push(pkg.clone());
        Some(UpdateStatus::Installing {
            current,
            total,
            package: pkg,
        })
    } else if lower.contains("installing")
        || lower.contains("upgrading")
        || lower.contains("checking keys")
        || lower.contains("checking integrity")
    {
        Some(UpdateStatus::Installing {
            current: 0,
            total: 0,
            package: "System".to_string(),
        })
    } else if lower.contains("there is nothing to do") {
        Some(UpdateStatus::NoUpdates)
    } else {
        None
    };

    if let Some(status) = new_status {
        let _ = sender
            .send(SystemUpdateProgress::StatusChange(status))
            .await;
    }
}

fn parse_install_progress(line: &str) -> Option<(usize, usize, String)> {
    let line = line.trim();
    if !line.starts_with('(') {
        return None;
    }

    let end_paren = line.find(')')?;
    let slash = line.find('/')?;

    if slash > end_paren {
        return None;
    }

    let current_str = line[1..slash].trim();
    let total_str = line[slash + 1..end_paren].trim();

    let current = current_str.parse().ok()?;
    let total = total_str.parse().ok()?;

    let rest = line[end_paren + 1..].trim();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let raw_pkg = parts.get(1).unwrap_or(&"unknown");
    let package = raw_pkg.trim_end_matches("...").to_string();

    Some((current, total, package))
}

// Parses "==> Making package: package_name version ..."
fn parse_building_package(line: &str) -> Option<String> {
    if line.starts_with("==> Making package:") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // parts[0] = "==>", parts[1] = "Making", parts[2] = "package:", parts[3] = package_name
        if parts.len() >= 4 {
            return Some(parts[3].to_string());
        }
    }
    None
}

fn parse_downloading_package(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.ends_with("downloading...") {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if !parts.is_empty() {
            return Some(parts[0].to_string());
        }
    }
    None
}

fn check_restart_required(packages: &[String]) -> bool {
    let critical_packages = [
        "linux",
        "linux-lts",
        "linux-zen",
        "linux-hardened",
        "systemd",
        "nvidia",
        "mesa",
        "amd-ucode",
        "intel-ucode",
        "glibc",
    ];

    packages.iter().any(|pkg| {
        critical_packages
            .iter()
            .any(|crit| pkg == *crit || pkg.starts_with(&format!("{}-", crit)))
    })
}

fn get_update_command() -> Result<UpdateCommand, String> {
    if !command_exists("sudo") {
        return Err("sudo is required for system updates".to_string());
    }

    let askpass_path = get_askpass_script_path()
        .map_err(|err| format!("Failed to write askpass helper: {}", err))?;
    let socket_path =
        get_socket_path().map_err(|err| format!("Failed to get socket path: {}", err))?;

    let mut env_vars = HashMap::new();
    env_vars.insert(
        "SUDO_ASKPASS".to_string(),
        askpass_path.to_string_lossy().to_string(),
    );
    env_vars.insert(
        "RHINCO_TV_ASKPASS_SOCKET".to_string(),
        socket_path.to_string_lossy().to_string(),
    );

    if let Some(helper) = detect_aur_helper() {
        if helper == "yay" {
            Ok((
                "yay".to_string(),
                vec![
                    "-Syu",
                    "--noconfirm",
                    "--answerdiff=None",
                    "--answerclean=None",
                    "--answeredit=None",
                    "--answerupgrade=None",
                    "--sudo",
                    "sudo",
                    "--sudoflags",
                    "-A",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                env_vars,
            ))
        } else {
            Ok((
                "paru".to_string(),
                vec![
                    "-Syu",
                    "--noconfirm",
                    "--skipreview",
                    "--sudo",
                    "sudo",
                    "--sudoflags",
                    "-A",
                ]
                .into_iter()
                .map(String::from)
                .collect(),
                env_vars,
            ))
        }
    } else if command_exists("pacman") {
        Ok((
            "sudo".to_string(),
            vec!["-A", "pacman", "-Syu", "--noconfirm"]
                .into_iter()
                .map(String::from)
                .collect(),
            env_vars,
        ))
    } else {
        Err("No supported package manager found".to_string())
    }
}

fn detect_aur_helper() -> Option<&'static str> {
    ["yay", "paru"]
        .into_iter()
        .find(|helper| command_exists(helper))
}

/// Returns true if system updates are supported on this system.
/// Checks for supported package managers and required helpers.
pub fn is_update_supported() -> bool {
    get_update_command().is_ok()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::futures::StreamExt;

    #[test]
    fn test_parse_install_progress_simple() {
        let line = "(1/5) installing firefox...";
        let result = parse_install_progress(line);
        assert_eq!(result, Some((1, 5, "firefox".to_string())));
    }

    #[test]
    fn test_parse_downloading() {
        let line = " linux-firmware-20230804.7be2766d-2-any downloading...";
        let result = parse_downloading_package(line);
        assert_eq!(
            result,
            Some("linux-firmware-20230804.7be2766d-2-any".to_string())
        );
    }

    #[test]
    fn test_parse_building_package() {
        let line = "==> Making package: topgrade-bin 16.8.0-1 (Sa 10 Jan 2026 13:23:20 CET)";
        let result = parse_building_package(line);
        assert_eq!(result, Some("topgrade-bin".to_string()));
    }

    #[tokio::test]
    async fn test_monitor_child_completes_and_captures_output() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg("echo 'hello world'; echo 'error line' >&2");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        let child = cmd.spawn().expect("Failed to spawn sh");

        let (mut sender, mut receiver) = iced::futures::channel::mpsc::channel(100);
        let mut updated_packages = Vec::new();

        // Run monitor - this should complete and not hang
        // We wrap it in a timeout to ensure the test fails fast if it hangs
        let monitor_future = monitor_child(child, &mut sender, &mut updated_packages);

        if let Err(_) =
            tokio::time::timeout(std::time::Duration::from_secs(2), monitor_future).await
        {
            panic!("monitor_child timed out - likely infinite loop bug");
        }

        // Drop sender to close channel so we can drain receiver
        drop(sender);

        let mut captured_logs = Vec::new();
        while let Some(progress) = receiver.next().await {
            if let SystemUpdateProgress::LogLine(line) = progress {
                captured_logs.push(line);
            }
        }

        assert!(captured_logs.contains(&"hello world".to_string()));
        assert!(captured_logs.contains(&"error line".to_string()));
    }

    #[tokio::test]
    async fn test_process_output_buffer_handles_carriage_returns() {
        let (mut sender, mut receiver) = iced::futures::channel::mpsc::channel(100);
        let mut updated_packages = Vec::new();

        // Simulating curl output: "progress 1\rprogress 2\rfinal\n"
        let mut buffer = Vec::from(b"progress 1\rprogress 2\rfinal\n" as &[u8]);

        process_output_buffer(&mut buffer, &mut sender, &mut updated_packages).await;

        drop(sender);

        let mut lines = Vec::new();
        while let Some(progress) = receiver.next().await {
            if let SystemUpdateProgress::LogLine(line) = progress {
                lines.push(line);
            }
        }

        assert_eq!(
            lines,
            vec![
                "progress 1".to_string(),
                "progress 2".to_string(),
                "final".to_string()
            ]
        );
        assert!(buffer.is_empty());
    }
}
