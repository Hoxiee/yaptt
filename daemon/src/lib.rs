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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct TestEnv {
        dir: TempDir,
        state: PathBuf,
        pid: PathBuf,
    }

    impl TestEnv {
        fn new() -> Self {
            let dir = TempDir::new().unwrap();
            let state = dir.path().join("state");
            let pid = dir.path().join("pid");
            Self { dir, state, pid }
        }

        fn state_path(&self) -> &Path {
            &self.state
        }

        fn pid_path(&self) -> &Path {
            &self.pid
        }
    }

    // ── State tests ──────────────────────────────────────────────────────

    #[test]
    fn test_read_state_missing_file() {
        let env = TestEnv::new();
        assert!(!read_state_at(env.state_path()));
    }

    #[test]
    fn test_read_state_active() {
        let env = TestEnv::new();
        fs::write(env.state_path(), "1").unwrap();
        assert!(read_state_at(env.state_path()));
    }

    #[test]
    fn test_read_state_inactive() {
        let env = TestEnv::new();
        fs::write(env.state_path(), "0").unwrap();
        assert!(!read_state_at(env.state_path()));
    }

    #[test]
    fn test_read_state_empty_file() {
        let env = TestEnv::new();
        fs::write(env.state_path(), "").unwrap();
        assert!(!read_state_at(env.state_path()));
    }

    #[test]
    fn test_read_state_garbage() {
        let env = TestEnv::new();
        fs::write(env.state_path(), "not a number").unwrap();
        assert!(!read_state_at(env.state_path()));
    }

    #[test]
    fn test_read_state_whitespace() {
        let env = TestEnv::new();
        fs::write(env.state_path(), "  1  \n").unwrap();
        assert!(read_state_at(env.state_path()));
    }

    #[test]
    fn test_read_state_newline_0() {
        let env = TestEnv::new();
        fs::write(env.state_path(), "0\n").unwrap();
        assert!(!read_state_at(env.state_path()));
    }

    #[test]
    fn test_write_state_active() {
        let env = TestEnv::new();
        write_state_at(env.state_path(), true);
        assert_eq!(fs::read_to_string(env.state_path()).unwrap(), "1");
    }

    #[test]
    fn test_write_state_inactive() {
        let env = TestEnv::new();
        write_state_at(env.state_path(), false);
        assert_eq!(fs::read_to_string(env.state_path()).unwrap(), "0");
    }

    #[test]
    fn test_write_state_overwrites() {
        let env = TestEnv::new();
        write_state_at(env.state_path(), true);
        write_state_at(env.state_path(), false);
        assert_eq!(fs::read_to_string(env.state_path()).unwrap(), "0");
    }

    #[test]
    fn test_state_roundtrip() {
        let env = TestEnv::new();
        write_state_at(env.state_path(), true);
        assert!(read_state_at(env.state_path()));
        write_state_at(env.state_path(), false);
        assert!(!read_state_at(env.state_path()));
    }

    // ── PID tests ────────────────────────────────────────────────────────

    #[test]
    fn test_read_pid_missing() {
        let env = TestEnv::new();
        assert_eq!(read_pid_at(env.pid_path()), None);
    }

    #[test]
    fn test_read_pid_valid() {
        let env = TestEnv::new();
        fs::write(env.pid_path(), "12345").unwrap();
        assert_eq!(read_pid_at(env.pid_path()), Some(12345));
    }

    #[test]
    fn test_read_pid_large_number() {
        let env = TestEnv::new();
        fs::write(env.pid_path(), "4294967295").unwrap();
        assert_eq!(read_pid_at(env.pid_path()), Some(4294967295));
    }

    #[test]
    fn test_read_pid_invalid() {
        let env = TestEnv::new();
        fs::write(env.pid_path(), "abc").unwrap();
        assert_eq!(read_pid_at(env.pid_path()), None);
    }

    #[test]
    fn test_read_pid_negative() {
        let env = TestEnv::new();
        fs::write(env.pid_path(), "-1").unwrap();
        assert_eq!(read_pid_at(env.pid_path()), None);
    }

    #[test]
    fn test_read_pid_empty() {
        let env = TestEnv::new();
        fs::write(env.pid_path(), "").unwrap();
        assert_eq!(read_pid_at(env.pid_path()), None);
    }

    #[test]
    fn test_read_pid_with_newline() {
        let env = TestEnv::new();
        fs::write(env.pid_path(), "999\n").unwrap();
        assert_eq!(read_pid_at(env.pid_path()), Some(999));
    }

    #[test]
    fn test_remove_pid() {
        let env = TestEnv::new();
        fs::write(env.pid_path(), "999").unwrap();
        remove_file_if_exists(env.pid_path());
        assert!(!env.pid_path().exists());
    }

    #[test]
    fn test_remove_pid_missing() {
        let env = TestEnv::new();
        remove_file_if_exists(env.pid_path()); // should not panic
    }

    // ── Event handling tests ──────────────────────────────────────────────

    #[test]
    fn test_handle_key_event_ptt_press() {
        let ptt_key = Key::new(183); // F13
        assert_eq!(handle_key_event(183, 1, ptt_key), Some(true));
    }

    #[test]
    fn test_handle_key_event_ptt_release() {
        let ptt_key = Key::new(183);
        assert_eq!(handle_key_event(183, 0, ptt_key), Some(false));
    }

    #[test]
    fn test_handle_key_event_ptt_repeat() {
        let ptt_key = Key::new(183);
        assert_eq!(handle_key_event(183, 2, ptt_key), None);
    }

    #[test]
    fn test_handle_key_event_other_key_press() {
        let ptt_key = Key::new(183);
        assert_eq!(handle_key_event(30, 1, ptt_key), None); // KEY_A
    }

    #[test]
    fn test_handle_key_event_other_key_release() {
        let ptt_key = Key::new(183);
        assert_eq!(handle_key_event(30, 0, ptt_key), None);
    }

    #[test]
    fn test_handle_key_event_grave_vs_f13() {
        let ptt_key = Key::new(183); // F13
        assert_eq!(handle_key_event(41, 1, ptt_key), None); // KEY_GRAVE = 41
    }

    #[test]
    fn test_handle_key_event_custom_key() {
        let custom = Key::new(59); // F1
        assert_eq!(handle_key_event(59, 1, custom), Some(true));
        assert_eq!(handle_key_event(183, 1, custom), None);
    }

    #[test]
    fn test_handle_key_event_high_value() {
        let ptt_key = Key::new(183);
        assert_eq!(handle_key_event(183, 100, ptt_key), None); // unusual value
    }

    #[test]
    fn test_handle_key_event_negative_value() {
        let ptt_key = Key::new(183);
        assert_eq!(handle_key_event(183, -1, ptt_key), None); // shouldn't happen but covered
    }

    // ── Config tests ──────────────────────────────────────────────────────

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.ptt_key, Key::new(183));
        assert_eq!(config.source, None);
    }

    #[test]
    fn test_config_custom() {
        let config = Config {
            ptt_key: Key::new(59),
            source: Some("my mic".into()),
        };
        assert_eq!(config.ptt_key, Key::new(59));
        assert_eq!(config.source.as_deref(), Some("my mic"));
    }

    #[test]
    fn test_config_clone() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_config_debug() {
        let config = Config::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("ptt_key"));
        assert!(debug.contains("source"));
    }

    #[test]
    fn test_config_equality() {
        let a = Config::default();
        let b = Config::default();
        let c = Config {
            ptt_key: Key::new(59),
            source: None,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ── Integration: state + pid together ─────────────────────────────────

    #[test]
    fn test_full_lifecycle() {
        let env = TestEnv::new();

        // Start: no state, no pid
        assert!(!read_state_at(env.state_path()));
        assert_eq!(read_pid_at(env.pid_path()), None);

        // Write PID
        fs::write(env.pid_path(), "1234").unwrap();
        assert_eq!(read_pid_at(env.pid_path()), Some(1234));

        // Activate
        write_state_at(env.state_path(), true);
        assert!(read_state_at(env.state_path()));

        // Toggle OFF
        write_state_at(env.state_path(), false);
        assert!(!read_state_at(env.state_path()));

        // Cleanup
        remove_file_if_exists(env.pid_path());
        remove_file_if_exists(env.state_path());
        assert!(!env.pid_path().exists());
        assert!(!env.state_path().exists());
    }

    #[test]
    fn test_concurrent_safe_state_writes() {
        let env = TestEnv::new();

        // Rapid toggles should not corrupt state
        for i in 0..100 {
            write_state_at(env.state_path(), i % 2 == 0);
        }
        // After even number of writes (0-99 = 100 writes), last was i=99 (odd) → false
        assert!(!read_state_at(env.state_path()));
    }
}
