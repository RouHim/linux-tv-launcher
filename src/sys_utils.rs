use std::path::PathBuf;
use std::time::Duration;
use std::{env, io, process, thread};

/// Restarts the current process
pub fn restart_process(current_executable: PathBuf) {
    println!(
        "Restarting {} in 3 seconds...",
        current_executable.display()
    );
    thread::sleep(Duration::from_secs(3));
    let err = exec(process::Command::new(current_executable.clone()).args(env::args().skip(1)));
    eprintln!(
        "Error: Failed to restart process {}: {}",
        current_executable.display(),
        err
    );
    std::process::exit(1);
}

/// Replaces the current process with a new one
/// This function is only available on Unix platforms
#[cfg(unix)]
fn exec(command: &mut process::Command) -> io::Error {
    use std::os::unix::process::CommandExt as _;
    // Completely replace the current process image. If successful, execution
    // of the current process stops here.
    command.exec()
}

#[cfg(windows)]
fn exec(command: &mut process::Command) -> io::Error {
    // On Windows, we cannot replace the current process, so we just spawn a new one
    // and exit the current one.
    match command.spawn() {
        Ok(_) => std::process::exit(0),
        Err(err) => err,
    }
}
