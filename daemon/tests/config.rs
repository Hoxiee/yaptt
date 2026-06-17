use evdev::Key;
use ptt_daemon::Config;

#[test]
fn default_values() {
    let c = Config::default();
    assert_eq!(c.ptt_key, Key::new(183));
    assert_eq!(c.source, None);
}

#[test]
fn custom_values() {
    let c = Config {
        ptt_key: Key::new(59),
        source: Some("my mic".into()),
    };
    assert_eq!(c.ptt_key, Key::new(59));
    assert_eq!(c.source.as_deref(), Some("my mic"));
}

#[test]
fn clone() {
    let a = Config::default();
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn equality() {
    let a = Config::default();
    let b = Config::default();
    let c = Config {
        ptt_key: Key::new(59),
        source: None,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn debug_output() {
    let c = Config::default();
    let d = format!("{c:?}");
    assert!(d.contains("ptt_key"));
    assert!(d.contains("source"));
}
