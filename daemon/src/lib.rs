use anyhow::{Context, Result};
use evdev::Key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const STATE_FILE: &str = "/tmp/ptt-state";
pub const PID_FILE: &str = "/tmp/ptt-daemon.pid";
pub const KEYD_CONFIG: &str = "/etc/keyd/default.conf";
pub const DEFAULT_CONFIG_DIR: &str = ".config/ptt";
pub const CONFIG_FILE: &str = "config.json";

// ── Key name mapping ─────────────────────────────────────────────────────────

pub fn key_name_to_code(name: &str) -> Option<u16> {
    let map = key_name_map();
    map.get(name).copied()
}

pub fn key_code_to_name(code: u16) -> Option<String> {
    let map = key_name_map();
    map.iter()
        .find(|(_, &c)| c == code)
        .map(|(name, _)| name.clone())
}

pub fn available_keys() -> Vec<String> {
    let map = key_name_map();
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort();
    keys
}

fn key_name_map() -> HashMap<String, u16> {
    let mut m = HashMap::new();
    // Special keys
    m.insert("grave".into(), 41);
    m.insert("esc".into(), 1);
    m.insert("tab".into(), 15);
    m.insert("capslock".into(), 58);
    m.insert("space".into(), 57);
    m.insert("enter".into(), 28);
    m.insert("backspace".into(), 14);
    // F keys (corrected codes)
    let f_keys = [
        (1, 59), (2, 60), (3, 61), (4, 62), (5, 63), (6, 64),
        (7, 65), (8, 66), (9, 67), (10, 68), (11, 87), (12, 88),
        (13, 183), (14, 184), (15, 185), (16, 186), (17, 187), (18, 188),
        (19, 189), (20, 190), (21, 191), (22, 192), (23, 193), (24, 194),
    ];
    for (num, code) in f_keys {
        m.insert(format!("f{num}"), code);
    }
    // Letters (actual Linux key codes)
    let letters = [
        ('a', 30), ('b', 48), ('c', 46), ('d', 32), ('e', 18), ('f', 33),
        ('g', 34), ('h', 35), ('i', 23), ('j', 36), ('k', 37), ('l', 38),
        ('m', 50), ('n', 49), ('o', 24), ('p', 25), ('q', 16), ('r', 19),
        ('s', 31), ('t', 20), ('u', 22), ('v', 47), ('w', 17), ('x', 45),
        ('y', 21), ('z', 44),
    ];
    for (c, code) in letters {
        m.insert(c.to_string(), code);
    }
    // Numbers (1=2 .. 0=11)
    for i in 1..=9 {
        m.insert(i.to_string(), 1 + i as u16);
    }
    m.insert("0".into(), 11);
    // Modifiers
    m.insert("leftctrl".into(), 29);
    m.insert("rightctrl".into(), 97);
    m.insert("leftshift".into(), 42);
    m.insert("rightshift".into(), 54);
    m.insert("leftalt".into(), 56);
    m.insert("rightalt".into(), 100);
    m.insert("leftmeta".into(), 125);
    m.insert("rightmeta".into(), 126);
    m
}

// ── Configuration ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PttConfig {
    pub ptt_key: String,
    pub remap_key: String,
    pub source: Option<String>,
}

impl Default for PttConfig {
    fn default() -> Self {
        Self {
            ptt_key: "grave".into(),
            remap_key: "f13".into(),
            source: None,
        }
    }
}

impl PttConfig {
    pub fn ptt_key_code(&self) -> Option<u16> {
        key_name_to_code(&self.ptt_key)
    }

    pub fn remap_key_name(&self) -> &str {
        &self.remap_key
    }
}

pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DEFAULT_CONFIG_DIR)
}

pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

pub fn load_config() -> PttConfig {
    load_config_at(&config_path())
}

pub fn load_config_at(path: &Path) -> PttConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_config(config: &PttConfig) -> Result<()> {
    save_config_at(config, &config_path())
}

pub fn save_config_at(config: &PttConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create config dir")?;
    }
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(path, json).context("Failed to write config")?;
    Ok(())
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

pub fn remap_key(source: &str, target: &str) -> Result<()> {
    remap_key_to(source, target, KEYD_CONFIG)
}

pub fn remap_key_to(source: &str, target: &str, config_path: &str) -> Result<()> {
    let config = format!("[ids]\n\n*\n\n[main]\n\n{source} = {target}\n");
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

pub fn ptt_activate_with_config(config: &PttConfig, state_path: &Path) -> Result<()> {
    remap_key(&config.ptt_key, &config.remap_key)?;
    wpctl_mute(true);
    write_state_at(state_path, true);
    Ok(())
}

pub fn ptt_deactivate_with_config(config: &PttConfig, state_path: &Path) -> Result<()> {
    remap_key(&config.ptt_key, &config.ptt_key)?;
    wpctl_mute(false);
    write_state_at(state_path, false);
    Ok(())
}

pub fn ptt_toggle_with_config(config: &PttConfig, state_path: &Path) -> Result<bool> {
    let active = read_state_at(state_path);
    if active {
        ptt_deactivate_with_config(config, state_path)?;
        Ok(false)
    } else {
        ptt_activate_with_config(config, state_path)?;
        Ok(true)
    }
}

