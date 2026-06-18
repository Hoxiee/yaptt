use evdev::Key;
use yaptt_daemon::handle_key_event;

const F13: Key = Key::new(183);
const GRAVE: Key = Key::new(41);
const F1: Key = Key::new(59);
const KEY_A: Key = Key::new(30);

#[test]
fn ptt_press() {
    assert_eq!(handle_key_event(F13.code(), 1, F13), Some(true));
}

#[test]
fn ptt_release() {
    assert_eq!(handle_key_event(F13.code(), 0, F13), Some(false));
}

#[test]
fn ptt_repeat_ignored() {
    assert_eq!(handle_key_event(F13.code(), 2, F13), None);
}

#[test]
fn other_key_press_ignored() {
    assert_eq!(handle_key_event(KEY_A.code(), 1, F13), None);
}

#[test]
fn other_key_release_ignored() {
    assert_eq!(handle_key_event(KEY_A.code(), 0, F13), None);
}

#[test]
fn grave_not_f13() {
    assert_eq!(handle_key_event(GRAVE.code(), 1, F13), None);
}

#[test]
fn custom_key_f1() {
    assert_eq!(handle_key_event(F1.code(), 1, F1), Some(true));
    assert_eq!(handle_key_event(F13.code(), 1, F1), None);
}

#[test]
fn unusual_value() {
    assert_eq!(handle_key_event(F13.code(), 100, F13), None);
}

#[test]
fn negative_value() {
    assert_eq!(handle_key_event(F13.code(), -1, F13), None);
}
