use procfs::process::Process;
use std::ffi::OsStr;
use std::time::{Duration, Instant};
use tracing::{info, warn, debug};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const STEAM_LAUNCH_TIMEOUT: Duration = Duration::from_secs(60);
const GAME_EXIT_GRACE_PERIOD: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub enum MonitorTarget {
    Pid(u32),
    SteamAppId(String),
}

pub async fn monitor_app_process(target: MonitorTarget) {
    info!("Starting process monitor for {:?}", target);

    match &target {
        MonitorTarget::Pid(pid) => monitor_pid(*pid).await,
        MonitorTarget::SteamAppId(appid) => monitor_steam_game(appid.clone()).await,
    }
    
    info!("Monitor task complete for {:?}", target);
}

async fn monitor_pid(pid: u32) {
    loop {
        if !is_process_running(pid) {
            info!("Process {} exited.", pid);
            break;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn monitor_steam_game(appid: String) {
    let start_time = Instant::now();
    let mut game_found_once = false;
    let mut last_seen_time = Instant::now();

    info!("Monitoring Steam AppId: {}", appid);

    loop {
        let is_running = is_steam_game_running(&appid);

        if is_running {
            if !game_found_once {
                info!("Steam game {} detected running.", appid);
                game_found_once = true;
            }
            last_seen_time = Instant::now();
        } else {
            // Not running currently
            if !game_found_once {
                // We haven't seen it start yet. Check timeout.
                if start_time.elapsed() > STEAM_LAUNCH_TIMEOUT {
                    warn!("Timed out waiting for Steam game {} to start.", appid);
                    break;
                }
            } else {
                // We saw it running before, but now it's gone.
                // Check grace period (in case of launcher -> game transition)
                if last_seen_time.elapsed() > GAME_EXIT_GRACE_PERIOD {
                    info!("Steam game {} exited (grace period expired).", appid);
                    break;
                } else {
                    debug!("Steam game {} disappeared, waiting grace period...", appid);
                }
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

fn is_process_running(pid: u32) -> bool {
    Process::new(pid as i32).is_ok()
}

fn is_steam_game_running(target_appid: &str) -> bool {
    let all_procs = match procfs::process::all_processes() {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to read processes: {}", e);
            return false;
        }
    };

    let target_key = OsStr::new("SteamAppId");
    let target_val = OsStr::new(target_appid);

    for p in all_procs {
        let process = match p {
            Ok(proc) => proc,
            Err(_) => continue,
        };

        if let Ok(environ) = process.environ() {
            if let Some(appid) = environ.get(target_key) {
                if appid == target_val {
                    return true;
                }
            }
        }
    }

    false
}