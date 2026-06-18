use yaptt_daemon::*;

#[test]
fn ptt_key_only_triggers_when_active() {
    let ptt_key = evdev::Key::KEY_GRAVE;

    assert_eq!(handle_key_event(41, 1, ptt_key), Some(true), "grave press should trigger");
    assert_eq!(handle_key_event(41, 0, ptt_key), Some(false), "grave release should trigger");
    assert_eq!(handle_key_event(41, 2, ptt_key), None, "grave repeat should not trigger");
    assert_eq!(handle_key_event(30, 1, ptt_key), None, "other key should not trigger");
    assert_eq!(handle_key_event(30, 0, ptt_key), None, "other key release should not trigger");
}

#[test]
fn ptt_key_code_matches_config() {
    let config = PttConfig {
        ptt_key: "grave".into(),
        remap_key: "f13".into(),
        source: None,
        ..Default::default()
    };
    assert_eq!(config.ptt_key_code(), Some(41));
    assert_eq!(config.remap_key_code(), Some(183));
}

#[test]
fn ptt_key_code_invalid_returns_none() {
    let config = PttConfig {
        ptt_key: "nonexistent".into(),
        remap_key: "f13".into(),
        source: None,
        ..Default::default()
    };
    assert_eq!(config.ptt_key_code(), None);
}

#[test]
fn ptt_toggle_state_file() {
    let dir = tempfile::tempdir().unwrap();
    let state_path = dir.path().join("state");
    let config = PttConfig::default();

    write_state_at(&state_path, false);
    assert!(!read_state_at(&state_path));

    ptt_activate_with_config(&config, &state_path).unwrap();
    assert!(read_state_at(&state_path));

    ptt_deactivate_with_config(&config, &state_path).unwrap();
    assert!(!read_state_at(&state_path));
}

#[test]
fn ptt_toggle_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let state_path = dir.path().join("state");
    let config = PttConfig::default();

    write_state_at(&state_path, false);
    let result = ptt_toggle_with_config(&config, &state_path).unwrap();
    assert!(result);
    assert!(read_state_at(&state_path));

    let result = ptt_toggle_with_config(&config, &state_path).unwrap();
    assert!(!result);
    assert!(!read_state_at(&state_path));
}
