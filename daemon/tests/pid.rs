use ptt_daemon::{read_pid_at, remove_file_if_exists};
use std::fs;
use tempfile::TempDir;

#[allow(dead_code)]
struct PidTest {
    dir: TempDir,
    path: std::path::PathBuf,
}

impl PidTest {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("pid");
        Self { dir, path }
    }
}

#[test]
fn missing_file() {
    let t = PidTest::new();
    assert_eq!(read_pid_at(&t.path), None);
}

#[test]
fn valid_pid() {
    let t = PidTest::new();
    fs::write(&t.path, "12345").unwrap();
    assert_eq!(read_pid_at(&t.path), Some(12345));
}

#[test]
fn large_pid() {
    let t = PidTest::new();
    fs::write(&t.path, "4294967295").unwrap();
    assert_eq!(read_pid_at(&t.path), Some(4294967295));
}

#[test]
fn invalid_content() {
    let t = PidTest::new();
    fs::write(&t.path, "abc").unwrap();
    assert_eq!(read_pid_at(&t.path), None);
}

#[test]
fn negative_number() {
    let t = PidTest::new();
    fs::write(&t.path, "-1").unwrap();
    assert_eq!(read_pid_at(&t.path), None);
}

#[test]
fn empty_file() {
    let t = PidTest::new();
    fs::write(&t.path, "").unwrap();
    assert_eq!(read_pid_at(&t.path), None);
}

#[test]
fn with_newline() {
    let t = PidTest::new();
    fs::write(&t.path, "999\n").unwrap();
    assert_eq!(read_pid_at(&t.path), Some(999));
}

#[test]
fn remove_existing() {
    let t = PidTest::new();
    fs::write(&t.path, "999").unwrap();
    remove_file_if_exists(&t.path);
    assert!(!t.path.exists());
}

#[test]
fn remove_missing() {
    let t = PidTest::new();
    remove_file_if_exists(&t.path);
}
