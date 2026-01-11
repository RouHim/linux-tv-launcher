use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct GamingSystemInfo {
    pub os_name: String,
    pub kernel_version: String,
    pub cpu_model: String,
    pub memory_total: String,
    pub memory_used: String,
    pub gpu_info: String,
    pub gpu_driver: String,
    pub vulkan_info: String,
    pub xdg_session_type: String,
    pub wine_version: String,
    pub proton_versions: Vec<String>,
}

pub fn fetch_system_info() -> GamingSystemInfo {
    let os_name = get_os_name();
    let kernel_version = get_kernel_version();
    let cpu_model = get_cpu_model();
    let (memory_total, memory_used) = get_memory_info();
    let (gpu_info, gpu_driver) = get_gpu_info();
    let vulkan_info = get_vulkan_info();
    let xdg_session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "Unknown".to_string());
    let wine_version = get_wine_version();
    let proton_versions = get_proton_versions();

    GamingSystemInfo {
        os_name,
        kernel_version,
        cpu_model,
        memory_total,
        memory_used,
        gpu_info,
        gpu_driver,
        vulkan_info,
        xdg_session_type,
        wine_version,
        proton_versions,
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
        // Look for VGA or 3D controller classes
        // lspci -mm output format is roughly:
        // 00:02.0 "VGA compatible controller" "Intel Corporation" "UHD Graphics 620" ...
        if line.contains("\"VGA") || line.contains("\"3D") {
            // Split by quotes to get fields
            // parts[0] is slot (no quotes), parts[1] is empty (between slot and quote),
            // parts[2] is Class, parts[4] is Vendor, parts[6] is Device
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() >= 7 {
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
        (gpus.join("\n"), driver_info)
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

fn get_wine_version() -> String {
    Command::new("wine")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "Not Installed".to_string())
}

fn get_proton_versions() -> Vec<String> {
    let mut versions = Vec::new();
    let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());

    let search_paths = vec![
        PathBuf::from(&home).join(".steam/steam/steamapps/common"),
        PathBuf::from(&home).join(".local/share/Steam/steamapps/common"),
        PathBuf::from(&home).join(".steam/root/compatibilitytools.d"),
    ];

    for path in search_paths {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.to_lowercase().contains("proton")
                        || file_name.contains("GE-Proton")
                    {
                        if !versions.contains(&file_name) {
                            versions.push(file_name);
                        }
                    }
                }
            }
        }
    }

    versions.sort();
    if versions.is_empty() {
        versions.push("None Found".to_string());
    }
    versions
}
