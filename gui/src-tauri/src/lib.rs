use ptt_daemon::{
    available_keys, load_config, save_config, wpctl_list_sources, PttConfig,
};
use std::fs;
use std::process::Command;

const STATE_FILE: &str = "/tmp/ptt-state";
const PID_FILE: &str = "/tmp/ptt-daemon.pid";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
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
fn get_config() -> PttConfig {
    load_config()
}

#[tauri::command]
fn save_config_command(config: PttConfig) -> Result<PttConfig, String> {
    save_config(&config).map_err(|e| format!("Failed to save: {e}"))?;
    Ok(config)
}

#[tauri::command]
fn get_keys() -> Vec<String> {
    available_keys()
}

#[tauri::command]
fn get_sources() -> Vec<String> {
    wpctl_list_sources()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_status,
            toggle,
            get_config,
            save_config_command,
            get_keys,
            get_sources
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
