//! Core library for the yaptt push-to-talk daemon.
//!
//! Provides configuration, state management, key mapping, device discovery,
//! and wpctl/pactl helpers for controlling PipeWire/PulseAudio microphone state.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;

// ── Constants ────────────────────────────────────────────────────────────────

pub const STATE_FILE: &str = "/tmp/ptt-state";
pub const TALKING_FILE: &str = "/tmp/ptt-talking";
pub const PID_FILE: &str = "/tmp/ptt-daemon.pid";
pub const DEFAULT_CONFIG_DIR: &str = ".config/yaptt";
pub const CONFIG_FILE: &str = "config.json";

// ── Key name mapping ─────────────────────────────────────────────────────────

/// Convert a key name (e.g. "grave", "f1", "leftctrl") to an evdev key code.
pub fn key_name_to_code(name: &str) -> Option<u16> {
    let map = key_name_map();
    map.get(name).copied()
}

/// Convert an evdev key code to a key name.
pub fn key_code_to_name(code: u16) -> Option<String> {
    let map = key_name_map();
    map.iter()
        .find(|(_, &c)| c == code)
        .map(|(name, _)| name.clone())
}

/// List all supported key names, sorted alphabetically.
pub fn available_keys() -> Vec<String> {
    let map = key_name_map();
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort();
    keys
}

fn key_name_map() -> HashMap<String, u16> {
    let mut m = HashMap::new();
    m.insert("grave".into(), 41);
    m.insert("esc".into(), 1);
    m.insert("tab".into(), 15);
    m.insert("capslock".into(), 58);
    m.insert("space".into(), 57);
    m.insert("enter".into(), 28);
    m.insert("backspace".into(), 14);
    let f_keys = [
        (1, 59),
        (2, 60),
        (3, 61),
        (4, 62),
        (5, 63),
        (6, 64),
        (7, 65),
        (8, 66),
        (9, 67),
        (10, 68),
        (11, 87),
        (12, 88),
        (13, 183),
        (14, 184),
        (15, 185),
        (16, 186),
        (17, 187),
        (18, 188),
        (19, 189),
        (20, 190),
        (21, 191),
        (22, 192),
        (23, 193),
        (24, 194),
    ];
    for (num, code) in f_keys {
        m.insert(format!("f{num}"), code);
    }
    let letters = [
        ('a', 30),
        ('b', 48),
        ('c', 46),
        ('d', 32),
        ('e', 18),
        ('f', 33),
        ('g', 34),
        ('h', 35),
        ('i', 23),
        ('j', 36),
        ('k', 37),
        ('l', 38),
        ('m', 50),
        ('n', 49),
        ('o', 24),
        ('p', 25),
        ('q', 16),
        ('r', 19),
        ('s', 31),
        ('t', 20),
        ('u', 22),
        ('v', 47),
        ('w', 17),
        ('x', 45),
        ('y', 21),
        ('z', 44),
    ];
    for (c, code) in letters {
        m.insert(c.to_string(), code);
    }
    for i in 1..=9 {
        m.insert(i.to_string(), 1 + i as u16);
    }
    m.insert("0".into(), 11);
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

/// Daemon configuration, loaded from `~/.config/yaptt/config.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PttConfig {
    /// Key that activates push-to-talk when held (e.g. "grave", "f5").
    pub ptt_key: String,
    /// Key code forwarded to apps while PTT key is held (e.g. "f13").
    pub remap_key: String,
    /// Optional audio source override (reserved for future use).
    pub source: Option<String>,
    /// Fade-out duration in milliseconds when releasing the PTT key.
    #[serde(default = "default_fade_duration_ms")]
    pub fade_duration_ms: u64,
}

fn default_fade_duration_ms() -> u64 {
    35
}

impl Default for PttConfig {
    fn default() -> Self {
        Self {
            ptt_key: "grave".into(),
            remap_key: "f13".into(),
            source: None,
            fade_duration_ms: default_fade_duration_ms(),
        }
    }
}

impl PttConfig {
    pub fn ptt_key_code(&self) -> Option<u16> {
        key_name_to_code(&self.ptt_key)
    }

    pub fn remap_key_code(&self) -> Option<u16> {
        key_name_to_code(&self.remap_key)
    }

    pub fn remap_key_name(&self) -> &str {
        &self.remap_key
    }
}

/// Returns `~/.config/yaptt/`.
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DEFAULT_CONFIG_DIR)
}

/// Returns `~/.config/yaptt/config.json`.
pub fn config_path() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

/// Load config from the default path, falling back to defaults on error.
pub fn load_config() -> PttConfig {
    load_config_at(&config_path())
}

/// Load config from an explicit path.
pub fn load_config_at(path: &Path) -> PttConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Save config to the default path.
pub fn save_config(config: &PttConfig) -> Result<()> {
    save_config_at(config, &config_path())
}

/// Save config to an explicit path, creating parent directories as needed.
pub fn save_config_at(config: &PttConfig, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create config dir")?;
    }
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    fs::write(path, json).context("Failed to write config")?;
    Ok(())
}

// ── State / PID ─────────────────────────────────────────────────────────────

/// Read PTT active state from a file. Returns `true` if the file contains "1".
pub fn read_state_at(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

/// Write PTT active state to a file.
pub fn write_state_at(path: &Path, active: bool) {
    let _ = fs::write(path, if active { "1" } else { "0" });
}

/// Read a PID from a file.
pub fn read_pid_at(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Write the current process PID to a file.
pub fn write_pid_at(path: &Path) -> Result<u32> {
    let pid = std::process::id();
    fs::write(path, pid.to_string()).context("Failed to write PID file")?;
    Ok(pid)
}

/// Remove a file if it exists (no error on missing).
pub fn remove_file_if_exists(path: &Path) {
    let _ = fs::remove_file(path);
}

pub fn read_state() -> bool {
    read_state_at(Path::new(STATE_FILE))
}

pub fn write_state(active: bool) {
    write_state_at(Path::new(STATE_FILE), active)
}

/// Write the "talking" indicator file (grave key is held).
pub fn write_talking(talking: bool) {
    let _ = fs::write(TALKING_FILE, if talking { "1" } else { "0" });
}

/// Remove the "talking" indicator file.
pub fn clear_talking() {
    let _ = fs::remove_file(TALKING_FILE);
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

// ── wpctl / pactl helpers ───────────────────────────────────────────────────

/// Mute or unmute the default audio source via `wpctl set-mute`.
pub fn wpctl_mute_default(mute: bool) {
    let state = if mute { "1" } else { "0" };
    let _ = Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", state])
        .output();
}

/// Set volume for a wpctl node (clamped to 0.0–1.0).
pub fn wpctl_set_volume(node: &str, vol: f32) {
    let vol = vol.clamp(0.0, 1.0);
    let _ = Command::new("wpctl")
        .args(["set-volume", node, &format!("{vol:.4}")])
        .output();
}

/// Get current volume for a wpctl node (0.0–1.0).
pub fn wpctl_get_volume(node: &str) -> Option<f32> {
    let output = Command::new("wpctl")
        .args(["get-volume", node])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for part in stdout.split_whitespace() {
        if let Ok(v) = part.parse::<f32>() {
            return Some(v);
        }
    }
    None
}

/// Get the default audio source name via `pactl get-default-source`.
pub fn pactl_get_default_source() -> Option<String> {
    let output = Command::new("pactl")
        .args(["get-default-source"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let name = stdout.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Set the default audio source via `pactl set-default-source`.
pub fn pactl_set_default_source(name: &str) -> Result<()> {
    let output = Command::new("pactl")
        .args(["set-default-source", name])
        .output()
        .context("Failed to run pactl set-default-source")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("pactl set-default-source failed: {}", stderr);
    }
    info!("Set default source to: {}", name);
    Ok(())
}

// ── Device discovery ─────────────────────────────────────────────────────────

/// Find all keyboard devices by parsing `/proc/bus/input/devices`.
///
/// Skips mice, system buttons, audio devices, and virtual devices.
/// Returns a list of `(event_path, device_name)` tuples.
pub fn find_keyboard_devices() -> Vec<(PathBuf, String)> {
    let mut devices = Vec::new();

    if let Ok(proc) = fs::read_to_string("/proc/bus/input/devices") {
        let mut current_name = String::new();
        let mut current_handlers = String::new();

        for line in proc.lines() {
            if line.starts_with('N') && line.contains("Name=") {
                current_name = line
                    .split_once("Name=")
                    .map(|(_, v)| v.trim_matches('"').to_string())
                    .unwrap_or_default();
            } else if line.starts_with('H') && line.contains("Handlers=") {
                current_handlers = line
                    .split_once("Handlers=")
                    .map(|(_, v)| v.trim().to_string())
                    .unwrap_or_default();
            } else if line.starts_with('I') && !line.contains("ID_") {
                let skip_names = [
                    "power button",
                    "video bus",
                    "pc speaker",
                    "hda nvidia",
                    "hd-audio",
                    "vicinae",
                    "keyd virtual",
                    "system control",
                    "consumer control",
                    "audio",
                ];
                let lower = current_name.to_lowercase();

                if current_handlers.contains("kbd")
                    && !current_handlers.contains("mouse")
                    && !skip_names.iter().any(|skip| lower.contains(skip))
                {
                    if let Some(event_name) = current_handlers
                        .split_whitespace()
                        .find(|h| h.starts_with("event"))
                    {
                        let path = PathBuf::from("/dev/input").join(event_name);
                        if path.exists() {
                            devices.push((path, current_name.clone()));
                        }
                    }
                }

                current_name.clear();
                current_handlers.clear();
            }
        }
    }

    devices
}

// ── Event helpers ────────────────────────────────────────────────────────────

use evdev::Key;
use input_linux::{EventKind, EventTime, InputEvent};

pub fn make_key_event(code: u16, value: i32) -> InputEvent {
    InputEvent {
        time: EventTime::new(0, 0),
        kind: EventKind::Key,
        code,
        value,
    }
}

pub fn make_syn_report() -> InputEvent {
    InputEvent {
        time: EventTime::new(0, 0),
        kind: EventKind::Synchronize,
        code: 0,
        value: 0,
    }
}

pub fn handle_key_event(code: u16, value: i32, ptt_key: Key) -> Option<bool> {
    let key = Key::new(code);
    if key == ptt_key {
        match value {
            1 => Some(true),
            0 => Some(false),
            _ => None,
        }
    } else {
        None
    }
}

// ── PTT toggle logic ─────────────────────────────────────────────────────────

pub fn ptt_activate_with_config(_config: &PttConfig, state_path: &Path) -> Result<()> {
    wpctl_mute_default(true);
    write_state_at(state_path, true);
    Ok(())
}

pub fn ptt_deactivate_with_config(_config: &PttConfig, state_path: &Path) -> Result<()> {
    wpctl_mute_default(false);
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
