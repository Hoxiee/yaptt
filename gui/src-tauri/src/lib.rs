use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;

const STATE_FILE: &str = "/tmp/ptt-state";
const PID_FILE: &str = "/tmp/ptt-daemon.pid";

#[derive(Debug, Serialize, Deserialize)]
struct PttStatus {
    active: bool,
    pid: Option<u32>,
}

#[tauri::command]
fn get_status() -> PttStatus {
    let active = fs::read_to_string(STATE_FILE)
        .map(|s| s.trim() == "1")
        .unwrap_or(false);

    let pid = fs::read_to_string(PID_FILE)
        .ok()
        .and_then(|s| s.trim().parse().ok());

    PttStatus { active, pid }
}

#[tauri::command]
fn toggle() -> Result<PttStatus, String> {
    let pid: u32 = fs::read_to_string(PID_FILE)
        .map_err(|e| format!("PID file: {e}"))?
        .trim()
        .parse()
        .map_err(|_| "Invalid PID".to_string())?;

    unsafe {
        libc::kill(pid as i32, libc::SIGUSR1);
    }

    std::thread::sleep(std::time::Duration::from_millis(200));

    Ok(get_status())
}

#[tauri::command]
fn get_sources() -> Vec<String> {
    let output = match Command::new("wpctl").args(["status"]).output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut sources = Vec::new();
    let mut in_sources = false;

    for line in stdout.lines() {
        if line.contains("Sources:") {
            in_sources = true;
            continue;
        }
        if in_sources {
            if line.contains("Audio/Source") || line.contains("Stream/Input") {
                if let Some(name) = line.split_whitespace().last() {
                    sources.push(name.to_string());
                }
            } else if !line.starts_with("  ") && !line.starts_with("├") && !line.starts_with("└")
            {
                break;
            }
        }
    }

    sources
}

#[tauri::command]
fn get_keys() -> Vec<String> {
    vec![
        "grave".into(),
        "f1".into(),
        "f2".into(),
        "f3".into(),
        "f4".into(),
        "f5".into(),
        "f6".into(),
        "f7".into(),
        "f8".into(),
        "f9".into(),
        "f10".into(),
        "f11".into(),
        "f12".into(),
        "f13".into(),
        "f14".into(),
        "f15".into(),
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            toggle,
            get_sources,
            get_keys
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
