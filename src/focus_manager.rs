use procfs::process::Process;
use std::ffi::OsStr;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const STEAM_LAUNCH_TIMEOUT: Duration = Duration::from_secs(60);
const GAME_EXIT_GRACE_PERIOD: Duration = Duration::from_secs(2); // Increased grace period for safety

#[derive(Debug, Clone)]
pub enum MonitorTarget {
    Pid(u32),
    SteamAppId(String),
    EnvVarEq(String, String),
    CmdLineContains(String),
    Any(Vec<MonitorTarget>),
}

pub async fn monitor_app_process(target: MonitorTarget) {
    info!("Starting process monitor for {:?}", target);

    let start_time = Instant::now();
    let mut game_found_once = false;
    let mut last_seen_time = Instant::now();

    loop {
        let is_running = check_target_running(&target);

        if is_running {
            if !game_found_once {
                info!("Target detected running: {:?}", target);
                game_found_once = true;
            }
            last_seen_time = Instant::now();
        } else {
            // Not running currently
            if !game_found_once {
                // We haven't seen it start yet. Check timeout.
                if start_time.elapsed() > STEAM_LAUNCH_TIMEOUT {
                    warn!("Timed out waiting for target to start: {:?}", target);
                    break;
                }
            }
            else {
                // We saw it running before, but now it's gone.
                // Check grace period
                if last_seen_time.elapsed() > GAME_EXIT_GRACE_PERIOD {
                    info!("Target exited (grace period expired): {:?}", target);
                    break;
                }
                else {
                    debug!("Target disappeared, waiting grace period... {:?}", target);
                }
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }

    info!("Monitor task complete for {:?}", target);
}

fn check_target_running(target: &MonitorTarget) -> bool {
    match target {
        MonitorTarget::Pid(pid) => is_process_running(*pid),
        MonitorTarget::SteamAppId(appid) => check_env_var("SteamAppId", appid),
        MonitorTarget::EnvVarEq(key, val) => check_env_var(key, val),
        MonitorTarget::CmdLineContains(pattern) => check_cmdline(pattern),
        MonitorTarget::Any(targets) => targets.iter().any(check_target_running),
    }
}

fn is_process_running(pid: u32) -> bool {
    Process::new(pid as i32).is_ok()
}

fn check_cmdline(pattern: &str) -> bool {
    let all_procs = match procfs::process::all_processes() {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to read processes: {}", e);
            return false;
        }
    };

    let pattern_lower = pattern.to_lowercase();

    for p in all_procs {
        let process = match p {
            Ok(proc) => proc,
            Err(_) => continue,
        };

        if let Ok(cmdline) = process.cmdline() {
            // Join args to form full command line
            let full_cmd = cmdline.join(" ").to_lowercase();
            if full_cmd.contains(&pattern_lower) {
                return true;
            }
        }
    }

    false
}

fn check_env_var(target_key_str: &str, target_val_str: &str) -> bool {
    let all_procs = match procfs::process::all_processes() {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to read processes: {}", e);
            return false;
        }
    };

    let target_key = OsStr::new(target_key_str);
    let target_val = OsStr::new(target_val_str);

    for p in all_procs {
        let process = match p {
            Ok(proc) => proc,
            Err(_) => continue,
        };

        if let Ok(environ) = process.environ() {
            if let Some(val) = environ.get(target_key) {
                if val == target_val {
                    return true;
                }
            }
        }
    }

    false
}