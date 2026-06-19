//! Toggle the PTT daemon on/off via SIGUSR1.
//!
//! Reads the daemon PID from `/tmp/ptt-daemon.pid`, sends SIGUSR1,
//! then shows a desktop notification with the new state.

use std::fs;
use std::process::Command;
use clap::Parser;
use yaptt_daemon::*;

#[derive(Parser)]
#[command(name = "yaptt-toggle", version, about = "Toggle the PTT daemon on/off")]
struct Cli {}

const STATE_FILE: &str = "/tmp/ptt-state";
const PID_FILE: &str = "/tmp/ptt-daemon.pid";

fn notify(title: &str, body: &str, icon: &str) {
    let _ = Command::new("notify-send")
        .args(["-a", "ptt", "-i", icon, "-t", "2000", title, body])
        .output();
}

fn main() {
    Cli::parse();

    let pid: u32 = match fs::read_to_string(PID_FILE) {
        Ok(content) => content.trim().parse().unwrap_or_else(|_| {
            eprintln!("Invalid PID file");
            std::process::exit(1);
        }),
        Err(_) => {
            eprintln!("PTT daemon is not running");
            std::process::exit(1);
        }
    };

    let state = fs::read_to_string(STATE_FILE)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0".to_string());

    unsafe {
        libc::kill(pid as i32, libc::SIGUSR1);
    }

    std::thread::sleep(std::time::Duration::from_millis(200));

    let config = load_config();
    if state == "0" {
        notify(
            "PTT ON",
            &format!("Hold {} to talk", config.ptt_key),
            "microphone-sensitivity-high",
        );
    } else {
        notify(
            "PTT OFF",
            &format!("{} works normally", config.ptt_key),
            "microphone-sensitivity-muted",
        );
    }
}
