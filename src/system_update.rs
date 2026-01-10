use crate::system_update_state::{SystemUpdateProgress, UpdateStatus};
use iced::futures::{SinkExt, Stream};
use std::env;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::{error, info, warn};

pub fn system_update_stream() -> impl Stream<Item = SystemUpdateProgress> {
    iced::stream::channel(
        100,
        |mut output: iced::futures::channel::mpsc::Sender<SystemUpdateProgress>| async move {
            send_status(&mut output, UpdateStatus::Starting).await;

            if !command_exists("pkexec") {
                send_failed(
                    &mut output,
                    "pkexec is required for system updates".to_string(),
                )
                .await;
                return;
            }

            let Some((program, args)) = get_update_command() else {
                send_failed(
                    &mut output,
                    "No supported package manager found".to_string(),
                )
                .await;
                return;
            };

            info!("Starting system update using: {} {:?}", program, args);

            if crate::osk::show() {
                info!("On-screen keyboard shown for password entry");
            }

            let mut cmd = Command::new("pkexec");
            cmd.arg(program).args(&args);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());
            cmd.kill_on_drop(true);

            let mut updated_packages: Vec<String> = Vec::new();

            match cmd.spawn() {
                Ok(mut child) => {
                    let mut stdout = match child.stdout.take() {
                        Some(stdout) => stdout,
                        None => {
                            send_failed(&mut output, "Failed to capture stdout".to_string()).await;
                            return;
                        }
                    };
                    let mut stderr = match child.stderr.take() {
                        Some(stderr) => stderr,
                        None => {
                            send_failed(&mut output, "Failed to capture stderr".to_string()).await;
                            return;
                        }
                    };

                    let mut stdout_buf = Vec::new();
                    let mut stderr_buf = Vec::new();
                    let mut read_buf_stdout = [0u8; 1024];
                    let mut read_buf_stderr = [0u8; 1024];
                    let mut stdout_done = false;
                    let mut stderr_done = false;
                    let mut child_status: Option<Result<std::process::ExitStatus, std::io::Error>> =
                        None;

                    loop {
                        tokio::select! {
                            res = stdout.read(&mut read_buf_stdout), if !stdout_done => {
                                match res {
                                    Ok(0) => {
                                        stdout_done = true;
                                        if let Some(msg) = flush_output_buffer(&mut stdout_buf, &mut output, &mut updated_packages).await {
                                            send_failed(&mut output, msg).await;
                                            return;
                                        }
                                    }
                                    Ok(n) => {
                                        stdout_buf.extend_from_slice(&read_buf_stdout[..n]);
                                        if let Some(msg) = process_output_buffer(&mut stdout_buf, &mut output, &mut updated_packages).await {
                                            send_failed(&mut output, msg).await;
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Error reading stdout: {}", e);
                                        send_failed(&mut output, format!("Error reading stdout: {}", e)).await;
                                        return;
                                    }
                                }
                            }
                            res = stderr.read(&mut read_buf_stderr), if !stderr_done => {
                                match res {
                                    Ok(0) => {
                                        stderr_done = true;
                                        if let Some(msg) = flush_output_buffer(&mut stderr_buf, &mut output, &mut updated_packages).await {
                                            send_failed(&mut output, msg).await;
                                            return;
                                        }
                                    }
                                    Ok(n) => {
                                        stderr_buf.extend_from_slice(&read_buf_stderr[..n]);
                                        if let Some(msg) = process_output_buffer(&mut stderr_buf, &mut output, &mut updated_packages).await {
                                            send_failed(&mut output, msg).await;
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Error reading stderr: {}", e);
                                        send_failed(&mut output, format!("Error reading stderr: {}", e)).await;
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
                            handle_child_exit(status, &mut output, &updated_packages).await;
                            return;
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("Failed to spawn update process: {}", e);
                    error!("{}", msg);
                    send_failed(&mut output, msg).await;
                }
            }
        },
    )
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

// Processes the buffer: extracts complete lines, parses them, and checks for prompt lines.
// Returns Some(error_message) if manual intervention is detected.
async fn process_output_buffer(
    buffer: &mut Vec<u8>,
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    updated_packages: &mut Vec<String>,
) -> Option<String> {
    while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
        let line_bytes: Vec<u8> = buffer.drain(..=pos).collect();
        let line = String::from_utf8_lossy(&line_bytes);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_interactive_prompt_line(trimmed) {
            return Some(format!(
                "Manual intervention required.\nPrompt: {}",
                trimmed
            ));
        }
        parse_output_line(trimmed, sender, updated_packages).await;
    }

    let remaining = String::from_utf8_lossy(buffer);
    if !remaining.is_empty() && !remaining.contains('\n') && is_interactive_prompt_line(&remaining)
    {
        return Some(format!(
            "Manual intervention required.\nPrompt: {}",
            remaining.trim()
        ));
    }
    None
}

async fn flush_output_buffer(
    buffer: &mut Vec<u8>,
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    updated_packages: &mut Vec<String>,
) -> Option<String> {
    if buffer.is_empty() {
        return None;
    }

    let remaining = String::from_utf8_lossy(buffer).to_string();
    let trimmed = remaining.trim();
    if trimmed.is_empty() {
        buffer.clear();
        return None;
    }

    if is_interactive_prompt_line(trimmed) {
        buffer.clear();
        return Some(format!(
            "Manual intervention required.\nPrompt: {}",
            trimmed
        ));
    }

    parse_output_line(trimmed, sender, updated_packages).await;
    buffer.clear();
    None
}

fn is_interactive_prompt_line(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }

    if t.starts_with("::") {
        return t.ends_with("[Y/n]") || t.ends_with("[y/N]");
    }

    if t.starts_with("Enter a selection") || t.starts_with("Enter a number") {
        return t.ends_with(':') || t.ends_with("):");
    }

    false
}

async fn parse_output_line(
    line: &str,
    sender: &mut iced::futures::channel::mpsc::Sender<SystemUpdateProgress>,
    updated_packages: &mut Vec<String>,
) {
    // Log output to application logger
    info!("{}", line);

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

fn get_update_command() -> Option<(&'static str, Vec<&'static str>)> {
    if let Some(helper) = detect_aur_helper() {
        if helper == "yay" {
            Some((
                "yay",
                vec![
                    "-Syu",
                    "--noconfirm",
                    "--answerdiff=None",
                    "--answerclean=None",
                ],
            ))
        } else if helper == "paru" {
            Some(("paru", vec!["-Syu", "--noconfirm", "--skipreview"]))
        } else {
            Some((helper, vec!["-Syu", "--noconfirm"]))
        }
    } else if command_exists("pacman") {
        Some(("pacman", vec!["-Syu", "--noconfirm"]))
    } else {
        warn!("No supported package manager found.");
        None
    }
}

fn detect_aur_helper() -> Option<&'static str> {
    ["yay", "paru"]
        .into_iter()
        .find(|helper| command_exists(helper))
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

    #[test]
    fn test_is_interactive_prompt() {
        assert!(is_interactive_prompt_line(
            ":: Proceed with installation? [Y/n]"
        ));
        assert!(is_interactive_prompt_line(":: Import PGP key 123? [y/N]"));
        assert!(is_interactive_prompt_line(
            "Enter a selection (default=all):"
        ));
        assert!(is_interactive_prompt_line("Enter a number (default=1):"));
        assert!(!is_interactive_prompt_line("installing package..."));
        assert!(!is_interactive_prompt_line(
            "Downloading (default=all): file"
        ));
        assert!(!is_interactive_prompt_line("random [Y/n]"));
    }
}
