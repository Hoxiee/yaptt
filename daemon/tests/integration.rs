use yaptt_daemon::{read_pid_at, read_state_at, remove_file_if_exists, write_state_at};
use std::fs;
use tempfile::TempDir;

#[allow(dead_code)]
struct LifecycleTest {
    dir: TempDir,
    state: std::path::PathBuf,
    pid: std::path::PathBuf,
}

impl LifecycleTest {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let state = dir.path().join("state");
        let pid = dir.path().join("pid");
        Self { dir, state, pid }
    }
}

#[test]
fn full_lifecycle() {
    let t = LifecycleTest::new();

    assert!(!read_state_at(&t.state));
    assert_eq!(read_pid_at(&t.pid), None);

    fs::write(&t.pid, "1234").unwrap();
    assert_eq!(read_pid_at(&t.pid), Some(1234));

    write_state_at(&t.state, true);
    assert!(read_state_at(&t.state));

    write_state_at(&t.state, false);
    assert!(!read_state_at(&t.state));

    remove_file_if_exists(&t.pid);
    remove_file_if_exists(&t.state);
    assert!(!t.pid.exists());
    assert!(!t.state.exists());
}

#[test]
fn toggle_sequence() {
    let t = LifecycleTest::new();

    write_state_at(&t.state, false);
    assert!(!read_state_at(&t.state));

    write_state_at(&t.state, true);
    assert!(read_state_at(&t.state));

    write_state_at(&t.state, false);
    assert!(!read_state_at(&t.state));
}

#[test]
fn state_survives_pid_change() {
    let t = LifecycleTest::new();

    write_state_at(&t.state, true);
    fs::write(&t.pid, "100").unwrap();
    assert!(read_state_at(&t.state));
    assert_eq!(read_pid_at(&t.pid), Some(100));

    fs::write(&t.pid, "200").unwrap();
    assert!(read_state_at(&t.state));
    assert_eq!(read_pid_at(&t.pid), Some(200));
}
