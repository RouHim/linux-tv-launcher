use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct DiskInfo {
    pub mount_point: String,
    pub size: String,
    pub used: String,
    pub usage_percent: String,
}

#[derive(Debug, Clone, Default)]
pub struct ZramInfo {
    pub enabled: bool,
    pub size: String,
    pub algorithm: String,
    pub used: String,
    pub usage_percent: String,
}

#[derive(Debug, Clone, Default)]
pub struct ControllerInfo {
    pub name: String,
    pub device_path: String,
}

#[derive(Debug, Clone, Default)]
pub struct KernelTweaks {
    pub vm_max_map_count: u64,
    pub vm_max_map_count_ok: bool,
    pub swappiness: u8,
    pub swappiness_ok: bool,
    pub clocksource: String,
    pub clocksource_ok: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GameModeInfo {
    pub available: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GamingSystemInfo {
    pub os_name: String,
    pub kernel_version: String,
    pub cpu_model: String,
    pub cpu_governor: String,
    pub memory_total: String,
    pub memory_used: String,
    pub gpu_info: String,
    pub gpu_driver: String,
    pub vulkan_info: String,
    pub xdg_session_type: String,
    pub wine_versions: Vec<(String, String)>,
    pub proton_versions: Vec<(String, String)>,
    pub disks: Vec<DiskInfo>,
    pub zram: ZramInfo,
    pub controllers: Vec<ControllerInfo>,
    pub kernel_tweaks: KernelTweaks,
    pub gamemode: GameModeInfo,
}

pub fn fetch_system_info() -> GamingSystemInfo {
    let os_name = get_os_name();
    let kernel_version = get_kernel_version();
    let cpu_model = get_cpu_model();
    let cpu_governor = get_cpu_governor();
    let (memory_total, memory_used) = get_memory_info();
    let (gpu_info, gpu_driver) = get_gpu_info();
    let vulkan_info = get_vulkan_info();
    let xdg_session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "Unknown".to_string());
    let wine_versions = get_wine_versions();
    let proton_versions = get_proton_versions();
    let disks = get_disk_info();
    let zram = get_zram_info();
    let controllers = get_controllers();
    let kernel_tweaks = get_kernel_tweaks();
    let gamemode = get_gamemode_info();

    GamingSystemInfo {
        os_name,
        kernel_version,
        cpu_model,
        cpu_governor,
        memory_total,
        memory_used,
        gpu_info,
        gpu_driver,
        vulkan_info,
        xdg_session_type,
        wine_versions,
        proton_versions,
        disks,
        zram,
        controllers,
        kernel_tweaks,
        gamemode,
    }
}

fn get_os_name() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("PRETTY_NAME=") {
                return line
                    .trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
    "Linux".to_string()
}

fn get_kernel_version() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Unknown".to_string())
}

fn get_cpu_model() -> String {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name") {
                return line
                    .split(':')
                    .nth(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
            }
        }
    }
    "Unknown".to_string()
}

fn get_memory_info() -> (String, String) {
    if let Ok(output) = Command::new("free").arg("-h").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.starts_with("Mem:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    return (parts[1].to_string(), parts[2].to_string());
                }
            }
        }
    }
    ("Unknown".to_string(), "Unknown".to_string())
}

fn get_gpu_info() -> (String, String) {
    let mut gpus = Vec::new();
    let mut driver_info = String::from("Unknown");

    // 1. Get all GPUs from lspci
    let lspci = Command::new("lspci")
        .arg("-mm") // Machine readable: "Slot" "Class" "Vendor" "Device" ...
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    for line in lspci.lines() {
        // Look for VGA, 3D, or Display controller classes
        // lspci -mm output format:
        // 00:02.0 "VGA compatible controller" "Intel Corporation" "HD Graphics 530" ...
        // 01:00.0 "Display controller" "Advanced Micro Devices, Inc. [AMD/ATI]" ...
        if line.contains("\"VGA") || line.contains("\"3D") || line.contains("\"Display") {
            // Split by quotes to get fields
            // parts[0] = "Slot "
            // parts[1] = "Class"
            // parts[2] = " "
            // parts[3] = "Vendor"
            // parts[4] = " "
            // parts[5] = "Device"
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() >= 6 {
                let vendor = parts[3];
                let model = parts[5];
                gpus.push(format!("{} {}", vendor, model));
            } else {
                // Fallback parsing if split fails
                gpus.push(line.replace("\"", "").to_string());
            }
        }
    }

    // 2. Get active driver/renderer from glxinfo
    if let Ok(output) = Command::new("glxinfo").arg("-B").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let line = line.trim();
            if line.starts_with("OpenGL version string:") {
                // Example: "4.6 (Compatibility Profile) Mesa 23.1.3"
                driver_info = line
                    .trim_start_matches("OpenGL version string:")
                    .trim()
                    .to_string();
            }
        }
    }

    if gpus.is_empty() {
        ("Unknown GPU".to_string(), driver_info)
    } else {
        let gpu_list = if gpus.len() == 1 {
            gpus[0].clone()
        } else {
            gpus.into_iter()
                .enumerate()
                .map(|(i, gpu)| format!("GPU {}: {}", i + 1, gpu))
                .collect::<Vec<_>>()
                .join("\n")
        };
        (gpu_list, driver_info)
    }
}

fn get_vulkan_info() -> String {
    if let Ok(output) = Command::new("vulkaninfo").arg("--summary").output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        // Look for deviceName under Devices
        let mut device_name = String::new();
        let mut api_version = String::new();

        for line in output_str.lines() {
            let line = line.trim();
            if line.starts_with("deviceName") {
                device_name = line.split('=').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("apiVersion") {
                api_version = line.split('=').nth(1).unwrap_or("").trim().to_string();
            }
        }

        if !device_name.is_empty() {
            return format!("{} (v{})", device_name, api_version);
        }
    }
    "Not Available".to_string()
}

fn get_wine_versions() -> Vec<(String, String)> {
    if let Ok(output) = Command::new("wine").arg("--version").output() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        vec![("Wine".to_string(), version)]
    } else {
        vec![]
    }
}

/// Read version from a Proton installation directory's version file
fn read_proton_version_file(dir: &std::path::Path) -> Option<String> {
    let version_file = dir.join("version");
    if let Ok(content) = fs::read_to_string(&version_file) {
        // Format: "1767307616 GE-Proton10-28" - take second part
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 2 {
            return Some(parts[1..].join(" "));
        } else if !content.trim().is_empty() {
            return Some(content.trim().to_string());
        }
    }
    None
}

/// Extract version from directory name as fallback
fn extract_version_from_name(name: &str) -> String {
    // Handle names like "GE-Proton10-28", "Proton 9.0", "Proton-8.0"
    if let Some(idx) = name.to_lowercase().find("proton") {
        let after_proton = &name[idx + 6..]; // Skip "proton"
        let version = after_proton.trim_start_matches(['-', ' ', '_']);
        if !version.is_empty() {
            return version.to_string();
        }
    }
    "Unknown".to_string()
}

fn get_proton_versions() -> Vec<(String, String)> {
    let mut versions = Vec::new();
    let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());

    // User home directories
    let mut search_paths = vec![
        PathBuf::from(&home).join(".steam/steam/steamapps/common"),
        PathBuf::from(&home).join(".local/share/Steam/steamapps/common"),
        PathBuf::from(&home).join(".steam/root/compatibilitytools.d"),
        PathBuf::from(&home).join(".steam/steam/compatibilitytools.d"),
        PathBuf::from(&home).join(".local/share/Steam/compatibilitytools.d"),
        PathBuf::from(&home).join(".steam/compatibilitytools.d"),
        PathBuf::from(&home).join(".local/share/Steam/compatibility-tools.d"),
        PathBuf::from(&home)
            .join(".var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d"),
        // Snap
        PathBuf::from(&home).join("snap/steam/common/.steam/steam/compatibilitytools.d"),
    ];

    // System-wide directories
    search_paths.push(PathBuf::from("/usr/share/steam/compatibilitytools.d"));
    search_paths.push(PathBuf::from("/usr/local/share/steam/compatibilitytools.d"));

    for path in search_paths {
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.to_lowercase().contains("proton") {
                        let entry_path = entry.path();
                        // Try to read version file first, fallback to extracting from name
                        let version = read_proton_version_file(&entry_path)
                            .unwrap_or_else(|| extract_version_from_name(&file_name));
                        versions.push((file_name, version));
                    }
                }
            }
        }
    }

    versions.sort_by(|a, b| a.0.cmp(&b.0));
    versions.dedup_by(|a, b| a.0 == b.0);
    versions
}

fn get_disk_info() -> Vec<DiskInfo> {
    let mut disks = Vec::new();

    // Use df to get disk usage, excluding virtual filesystems
    if let Ok(output) = Command::new("df")
        .args([
            "-h",
            "--output=target,size,used,pcent",
            "-x",
            "tmpfs",
            "-x",
            "devtmpfs",
            "-x",
            "squashfs",
            "-x",
            "overlay",
            "-x",
            "efivarfs",
        ])
        .output()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines().skip(1) {
            // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let mount_point = parts[0].to_string();
                // Skip some system mounts that aren't useful for gaming
                if mount_point.starts_with("/snap")
                    || mount_point.starts_with("/boot")
                    || mount_point == "/efi"
                {
                    continue;
                }
                disks.push(DiskInfo {
                    mount_point,
                    size: parts[1].to_string(),
                    used: parts[2].to_string(),
                    usage_percent: parts[3].to_string(),
                });
            }
        }
    }

    disks
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

fn get_zram_info() -> ZramInfo {
    // Check if any zram device exists
    let zram_path = PathBuf::from("/sys/block/zram0");
    if !zram_path.exists() {
        return ZramInfo::default();
    }

    // Read disksize
    let disksize = fs::read_to_string(zram_path.join("disksize"))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);

    if disksize == 0 {
        return ZramInfo::default();
    }

    // Read compression algorithm (format: "lzo-rle lzo lz4 [zstd] deflate" - active one in brackets)
    let algorithm = fs::read_to_string(zram_path.join("comp_algorithm"))
        .ok()
        .and_then(|s| {
            // Find the algorithm in brackets [algo]
            if let Some(start) = s.find('[') {
                if let Some(end) = s.find(']') {
                    return Some(s[start + 1..end].to_string());
                }
            }
            // Fallback: just return the first algorithm
            s.split_whitespace().next().map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    // Check if zram is active in swap and get usage from /proc/swaps
    let mut used_kb: u64 = 0;
    let mut total_kb: u64 = 0;
    if let Ok(swaps) = fs::read_to_string("/proc/swaps") {
        for line in swaps.lines() {
            if line.contains("/dev/zram") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    total_kb = parts[2].parse().unwrap_or(0);
                    used_kb = parts[3].parse().unwrap_or(0);
                    break;
                }
            }
        }
    }

    let usage_percent = if total_kb > 0 {
        format!("{}%", (used_kb * 100) / total_kb)
    } else {
        "0%".to_string()
    };

    ZramInfo {
        enabled: true,
        size: format_bytes(disksize),
        algorithm,
        used: format_bytes(used_kb * 1024),
        usage_percent,
    }
}

fn get_cpu_governor() -> String {
    // Read governor from first CPU (they're usually all the same)
    if let Ok(governor) =
        fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    {
        return governor.trim().to_string();
    }
    "Unknown".to_string()
}

fn get_controllers() -> Vec<ControllerInfo> {
    let mut controllers = Vec::new();

    // Check /dev/input for joystick devices
    if let Ok(entries) = fs::read_dir("/dev/input") {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Look for js* devices (joysticks) and event* devices
            if file_name.starts_with("js") {
                let device_path = entry.path().to_string_lossy().to_string();

                // Try to get the device name from sysfs
                let js_num = file_name.trim_start_matches("js");
                let name_path = format!("/sys/class/input/js{}/device/name", js_num);

                let name = fs::read_to_string(&name_path)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| file_name.clone());

                controllers.push(ControllerInfo { name, device_path });
            }
        }
    }

    // Sort by device path for consistent ordering
    controllers.sort_by(|a, b| a.device_path.cmp(&b.device_path));
    controllers
}

fn get_kernel_tweaks() -> KernelTweaks {
    // vm.max_map_count - recommended: 2147483642 (SteamOS default), minimum good: 1048576 (Arch default)
    let vm_max_map_count = fs::read_to_string("/proc/sys/vm/max_map_count")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    let vm_max_map_count_ok = vm_max_map_count >= 1048576;

    // Swappiness - for gaming, lower is better (10 recommended)
    let swappiness = fs::read_to_string("/proc/sys/vm/swappiness")
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(60);
    let swappiness_ok = swappiness <= 10;

    // Clocksource - TSC is fastest, HPET/acpi_pm are slower
    let clocksource =
        fs::read_to_string("/sys/devices/system/clocksource/clocksource0/current_clocksource")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
    let clocksource_ok = clocksource == "tsc";

    KernelTweaks {
        vm_max_map_count,
        vm_max_map_count_ok,
        swappiness,
        swappiness_ok,
        clocksource,
        clocksource_ok,
    }
}

fn get_gamemode_info() -> GameModeInfo {
    // Check if gamemoded binary is available
    let available = Command::new("which")
        .arg("gamemoded")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    // Check if gamemode is currently active by querying gamemoded
    // gamemoded --status returns 0 if active, 1 if inactive
    let active = if available {
        Command::new("gamemoded")
            .arg("--status")
            .output()
            .map(|o| {
                // Status code: 0 = active, other = inactive
                // Output contains "gamemode is active" or "gamemode is inactive"
                let output = String::from_utf8_lossy(&o.stdout);
                output.to_lowercase().contains("active")
                    && !output.to_lowercase().contains("inactive")
            })
            .unwrap_or(false)
    } else {
        false
    };

    GameModeInfo { available, active }
}
