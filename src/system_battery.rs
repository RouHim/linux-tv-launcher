use gilrs::PowerInfo;
use std::fs;
use std::path::Path;

pub fn read_system_battery() -> Option<PowerInfo> {
    let power_supply_dir = Path::new("/sys/class/power_supply");

    let mut battery_paths: Vec<_> = fs::read_dir(power_supply_dir)
        .ok()?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?.to_string();
            if name.starts_with("BAT") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // Sort to ensure deterministic selection (e.g. BAT0 before BAT1)
    battery_paths.sort();

    battery_paths
        .into_iter()
        .find_map(|path| read_battery_info(&path))
}

fn read_battery_info(path: &Path) -> Option<PowerInfo> {
    let capacity = fs::read_to_string(path.join("capacity"))
        .ok()?
        .trim()
        .parse::<u8>()
        .unwrap_or(0);

    let status = fs::read_to_string(path.join("status"))
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    match status.as_str() {
        "Charging" => Some(PowerInfo::Charging(capacity)),
        "Discharging" => Some(PowerInfo::Discharging(capacity)),
        "Full" => Some(PowerInfo::Charged),
        "Not charging" if capacity > 90 => Some(PowerInfo::Charged),
        "Not charging" => Some(PowerInfo::Charging(capacity)),
        _ => Some(PowerInfo::Discharging(capacity)),
    }
}
