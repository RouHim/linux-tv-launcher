use procfs::process::Process;
use std::ffi::OsStr;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const POLL_INTERVAL_FAST: Duration = Duration::from_millis(250);
const POLL_INTERVAL_SLOW: Duration = Duration::from_millis(1000);
const STEAM_LAUNCH_TIMEOUT: Duration = Duration::from_secs(60);
const GAME_EXIT_GRACE_PERIOD_LONG: Duration = Duration::from_secs(10);
const GAME_EXIT_GRACE_PERIOD_SHORT: Duration = Duration::from_millis(500);
const STABLE_RUN_THRESHOLD: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub enum MonitorTarget {
    Pid(u32),
    SteamAppId(String),
    EnvVarEq(String, String),
    CmdLineContains(String),
    Any(Vec<MonitorTarget>),
}

pub async fn monitor_app_process(target: MonitorTarget) {
    let start_time = Instant::now();
    let mut game_found_once = false;
    let mut first_seen_time: Option<Instant> = None;
    let mut last_seen_time = Instant::now();
    let mut current_game_pid: Option<u32> = None;

    // Log the monitoring start
    info!(?target, "Starting monitoring");

    loop {
        let mut is_running = false;

        // 1. Fast Path: Check locked PID if we have one
        if let Some(pid) = current_game_pid {
            if is_process_running(pid) {
                is_running = true;
            } else {
                // PID died, reset lock and fall through to full scan
                info!(pid, "Locked PID exited. Scanning...");
                current_game_pid = None;
            }
        }

        // 2. Slow Path: Full system scan if not running (or just lost PID)
        if !is_running {
            let mut process_cache: Option<Vec<Process>> = None;
            if let Some(pid) = check_target_running(&target, &mut process_cache) {
                is_running = true;
                // Lock onto this new PID
                current_game_pid = Some(pid);
                info!(pid, "Found/Relocked PID");
            }
        }

        if is_running {
            if !game_found_once {
                info!("Game started/detected!");
                game_found_once = true;
                first_seen_time = Some(Instant::now());
            }
            last_seen_time = Instant::now();
        } else {
            // Not running currently
            if !game_found_once {
                // Launch Phase: Check timeout
                if start_time.elapsed() > STEAM_LAUNCH_TIMEOUT {
                    warn!("Launch timeout exceeded. Giving up.");
                    break;
                }
            } else {
                // Exit Phase: Check adaptive grace period
                let total_runtime =
                    last_seen_time.duration_since(first_seen_time.unwrap_or(last_seen_time));

                let grace_period = if total_runtime > STABLE_RUN_THRESHOLD {
                    GAME_EXIT_GRACE_PERIOD_SHORT
                } else {
                    GAME_EXIT_GRACE_PERIOD_LONG
                };

                if last_seen_time.elapsed() > grace_period {
                    info!(?total_runtime, "Game exited (grace period expired).");
                    break;
                }
            }
        }

        // Adaptive polling interval
        // Keep fast interval if we've seen the game at least once (Exit Phase),
        // so we don't overshoot the short grace period.
        let interval = if is_running || game_found_once {
            POLL_INTERVAL_FAST
        } else {
            POLL_INTERVAL_SLOW
        };

        tokio::time::sleep(interval).await;
    }
}

fn check_target_running(
    target: &MonitorTarget,
    process_cache: &mut Option<Vec<Process>>,
) -> Option<u32> {
    match target {
        MonitorTarget::Pid(pid) => {
            if is_process_running(*pid) {
                Some(*pid)
            } else {
                None
            }
        }
        MonitorTarget::SteamAppId(appid) => {
            check_env_var("SteamAppId", appid, get_processes(process_cache))
        }
        MonitorTarget::EnvVarEq(key, val) => check_env_var(key, val, get_processes(process_cache)),
        MonitorTarget::CmdLineContains(pattern) => {
            check_cmdline(pattern, get_processes(process_cache))
        }
        MonitorTarget::Any(targets) => targets
            .iter()
            .find_map(|t| check_target_running(t, process_cache)),
    }
}

fn get_processes(cache: &mut Option<Vec<Process>>) -> &[Process] {
    cache.get_or_insert_with(|| {
        procfs::process::all_processes()
            .map(|iter| iter.filter_map(|p| p.ok()).collect())
            .unwrap_or_default()
    })
}

fn is_process_running(pid: u32) -> bool {
    Process::new(pid as i32)
        .and_then(|p| p.stat())
        .map(|stat| stat.state != 'Z')
        .unwrap_or(false)
}

fn is_valid_search_candidate(process: &Process) -> bool {
    if let Ok(stat) = process.stat() {
        if stat.state == 'Z' {
            return false;
        }
        // Skip common helper processes to avoid false positives
        let name = stat.comm.to_lowercase();
        if matches!(
            name.as_str(),
            "steam" | "steamwebhelper" | "gameoverlayui" | "pressure-vessel"
        ) {
            return false;
        }
        return true;
    }
    false
}

fn check_cmdline(pattern: &str, processes: &[Process]) -> Option<u32> {
    let pattern_lower = pattern.to_lowercase();

    for process in processes.iter().filter(|p| is_valid_search_candidate(p)) {
        if let Ok(cmdline) = process.cmdline() {
            // Join args to form full command line
            let full_cmd = cmdline.join(" ").to_lowercase();
            if full_cmd.contains(&pattern_lower) {
                return Some(process.pid as u32);
            }
        }
    }

    None
}

fn check_env_var(target_key_str: &str, target_val_str: &str, processes: &[Process]) -> Option<u32> {
    let target_key = OsStr::new(target_key_str);
    let target_val = OsStr::new(target_val_str);

    for process in processes.iter().filter(|p| is_valid_search_candidate(p)) {
        if let Ok(environ) = process.environ() {
            if let Some(val) = environ.get(target_key) {
                if val == target_val {
                    return Some(process.pid as u32);
                }
            }
        }
    }

    None
}
