use std::fs;
use tempfile::TempDir;
use yaptt_daemon::{read_state_at, write_state_at};

#[allow(dead_code)]
struct StateTest {
    dir: TempDir,
    path: std::path::PathBuf,
}

impl StateTest {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state");
        Self { dir, path }
    }
}

#[test]
fn missing_file_returns_false() {
    let t = StateTest::new();
    assert!(!read_state_at(&t.path));
}

#[test]
fn active_state() {
    let t = StateTest::new();
    fs::write(&t.path, "1").unwrap();
    assert!(read_state_at(&t.path));
}

#[test]
fn inactive_state() {
    let t = StateTest::new();
    fs::write(&t.path, "0").unwrap();
    assert!(!read_state_at(&t.path));
}

#[test]
fn empty_file() {
    let t = StateTest::new();
    fs::write(&t.path, "").unwrap();
    assert!(!read_state_at(&t.path));
}

#[test]
fn garbage_content() {
    let t = StateTest::new();
    fs::write(&t.path, "not a number").unwrap();
    assert!(!read_state_at(&t.path));
}

#[test]
fn whitespace_around_one() {
    let t = StateTest::new();
    fs::write(&t.path, "  1  \n").unwrap();
    assert!(read_state_at(&t.path));
}

#[test]
fn newline_after_zero() {
    let t = StateTest::new();
    fs::write(&t.path, "0\n").unwrap();
    assert!(!read_state_at(&t.path));
}

#[test]
fn write_active() {
    let t = StateTest::new();
    write_state_at(&t.path, true);
    assert_eq!(fs::read_to_string(&t.path).unwrap(), "1");
}

#[test]
fn write_inactive() {
    let t = StateTest::new();
    write_state_at(&t.path, false);
    assert_eq!(fs::read_to_string(&t.path).unwrap(), "0");
}

#[test]
fn overwrite() {
    let t = StateTest::new();
    write_state_at(&t.path, true);
    write_state_at(&t.path, false);
    assert_eq!(fs::read_to_string(&t.path).unwrap(), "0");
}

#[test]
fn roundtrip() {
    let t = StateTest::new();
    write_state_at(&t.path, true);
    assert!(read_state_at(&t.path));
    write_state_at(&t.path, false);
    assert!(!read_state_at(&t.path));
}

#[test]
fn rapid_toggles() {
    let t = StateTest::new();
    for i in 0..100 {
        write_state_at(&t.path, i % 2 == 0);
    }
    assert!(!read_state_at(&t.path));
}
