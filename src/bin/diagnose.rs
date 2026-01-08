use std::env;
use procfs::process::all_processes;

fn main() {
    let args: Vec<String> = env::args().collect();
    let keywords = if args.len() > 1 {
        args[1..].to_vec()
    } else {
        vec![
            "speed".to_string(), 
            "underground".to_string(), 
            "heroic".to_string(), 
            "ulz7kk49ffuycxlkw2uavg".to_string(),
            "wine".to_string()
        ]
    };

    println!("Scanning processes for keywords: {:?}", keywords);

    let procs = all_processes().expect("Failed to list processes");
    for p in procs {
        let process = match p {
            Ok(p) => p,
            Err(_) => continue,
        };

        let pid = process.pid;
        let cmdline = match process.cmdline() {
            Ok(c) => c.join(" "),
            Err(_) => String::new(),
        };
        let cmdline_lower = cmdline.to_lowercase();
        
        // Also check executable name
        let exe = match process.exe() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => String::new(),
        };

        let mut matched = false;
        for k in &keywords {
            if cmdline_lower.contains(&k.to_lowercase()) || exe.to_lowercase().contains(&k.to_lowercase()) {
                matched = true;
                break;
            }
        }

        if matched {
            println!("--------------------------------------------------");
            println!("PID: {}", pid);
            println!("CMD: {}", cmdline);
            println!("EXE: {}", exe);
            
            if let Ok(environ) = process.environ() {
                println!("ENV:");
                for (key, val) in environ {
                    let key_str = key.to_string_lossy().to_string().to_uppercase();
                    if key_str.contains("APP") || key_str.contains("ID") || key_str.contains("HEROIC") || key_str.contains("GAME") {
                         println!("  {} = {:?}", key.to_string_lossy(), val.to_string_lossy());
                    }
                }
            }
        }
    }
}
