use std::fs;
use std::process::Command;

const STATE_FILE: &str = "/tmp/ptt-state";
const PID_FILE: &str = "/tmp/ptt-daemon.pid";

fn notify(title: &str, body: &str, icon: &str) {
    let _ = Command::new("notify-send")
        .args(["-a", "ptt", "-i", icon, "-t", "2000", title, body])
        .output();
}

fn main() {
    let pid = match fs::read_to_string(PID_FILE) {
        Ok(content) => content.trim().parse::<u32>().unwrap_or_else(|_| {
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

    // Send SIGUSR1
    unsafe {
        libc::kill(pid as i32, libc::SIGUSR1);
    }

    // Wait a bit for daemon to process
    std::thread::sleep(std::time::Duration::from_millis(200));

    if state == "0" {
        notify("PTT ON", "Hold Tilde to talk", "microphone-sensitivity-high");
    } else {
        notify("PTT OFF", "Tilde works normally", "microphone-sensitivity-muted");
    }
}
