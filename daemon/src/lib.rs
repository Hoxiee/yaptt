use anyhow::{Context, Result};
use evdev::Key;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const STATE_FILE: &str = "/tmp/ptt-state";
pub const PID_FILE: &str = "/tmp/ptt-daemon.pid";
pub const KEYD_CONFIG: &str = "/etc/keyd/default.conf";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub ptt_key: Key,
    pub source: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ptt_key: Key::new(183), // KEY_F13
            source: None,
        }
    }
}

// ── State / PID ─────────────────────────────────────────────────────────────

pub fn read_state_at(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

pub fn write_state_at(path: &Path, active: bool) {
    let _ = fs::write(path, if active { "1" } else { "0" });
}

pub fn read_pid_at(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

pub fn write_pid_at(path: &Path) -> Result<u32> {
    let pid = std::process::id();
    fs::write(path, pid.to_string()).context("Failed to write PID file")?;
    Ok(pid)
}

pub fn remove_file_if_exists(path: &Path) {
    let _ = fs::remove_file(path);
}

// Convenience wrappers using default paths
pub fn read_state() -> bool {
    read_state_at(Path::new(STATE_FILE))
}

pub fn write_state(active: bool) {
    write_state_at(Path::new(STATE_FILE), active)
}

pub fn read_pid() -> Option<u32> {
    read_pid_at(Path::new(PID_FILE))
}

pub fn write_pid() -> Result<u32> {
    write_pid_at(Path::new(PID_FILE))
}

pub fn remove_pid() {
    remove_file_if_exists(Path::new(PID_FILE))
}

// ── keyd / wpctl ─────────────────────────────────────────────────────────────

pub fn remap_grave(target: &str) -> Result<()> {
    remap_grave_to(target, KEYD_CONFIG)
}

pub fn remap_grave_to(target: &str, config_path: &str) -> Result<()> {
    let config = format!("[ids]\n\n*\n\n[main]\n\ngrave = {target}\n");
    let mut child = Command::new("sudo")
        .args(["-n", "tee", config_path])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn sudo tee")?;
    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(config.as_bytes())
            .context("Failed to write config")?;
    }
    child.wait().context("Failed to wait for sudo tee")?;
    Command::new("sudo")
        .args(["-n", "keyd", "reload"])
        .output()
        .context("Failed to reload keyd")?;
    Ok(())
}

pub fn wpctl_mute(mute: bool) {
    let state = if mute { "1" } else { "0" };
    let _ = Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", state])
        .output();
}

pub fn wpctl_get_mute() -> Option<bool> {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(stdout.contains("[MUTED]"))
}

pub fn wpctl_list_sources() -> Vec<String> {
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
            } else if !line.starts_with("  ") && !line.starts_with('├') && !line.starts_with('└')
            {
                break;
            }
        }
    }
    sources
}

// ── Device discovery ─────────────────────────────────────────────────────────

pub fn find_keyd_keyboard() -> Result<PathBuf> {
    let devices = evdev::enumerate();
    for (path, device) in devices {
        if let Some(name) = device.name() {
            if name.contains("keyd virtual keyboard") {
                return Ok(path);
            }
        }
    }
    anyhow::bail!("keyd virtual keyboard not found")
}

pub fn is_keyd_running() -> bool {
    Command::new("pgrep")
        .args(["-x", "keyd"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── Event handling ───────────────────────────────────────────────────────────

pub fn handle_key_event(code: u16, value: i32, ptt_key: Key) -> Option<bool> {
    let key = Key::new(code);
    if key == ptt_key {
        match value {
            1 => Some(true),  // pressed → unmute
            0 => Some(false), // released → mute
            _ => None,        // repeat etc.
        }
    } else {
        None
    }
}

// ── PTT toggle logic ─────────────────────────────────────────────────────────

pub fn ptt_activate_at(state_path: &Path) -> Result<()> {
    remap_grave("f13")?;
    wpctl_mute(true);
    write_state_at(state_path, true);
    Ok(())
}

pub fn ptt_deactivate_at(state_path: &Path) -> Result<()> {
    remap_grave("grave")?;
    wpctl_mute(false);
    write_state_at(state_path, false);
    Ok(())
}

pub fn ptt_toggle_at(state_path: &Path) -> Result<bool> {
    let active = read_state_at(state_path);
    if active {
        ptt_deactivate_at(state_path)?;
        Ok(false)
    } else {
        ptt_activate_at(state_path)?;
        Ok(true)
    }
}

