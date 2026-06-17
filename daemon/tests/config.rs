use ptt_daemon::PttConfig;

#[test]
fn default_values() {
    let c = PttConfig::default();
    assert_eq!(c.ptt_key, "grave");
    assert_eq!(c.remap_key, "f13");
    assert_eq!(c.source, None);
}

#[test]
fn custom_values() {
    let c = PttConfig {
        ptt_key: "f1".into(),
        remap_key: "f14".into(),
        source: Some("my mic".into()),
    };
    assert_eq!(c.ptt_key, "f1");
    assert_eq!(c.remap_key, "f14");
    assert_eq!(c.source.as_deref(), Some("my mic"));
}

#[test]
fn clone() {
    let a = PttConfig::default();
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn equality() {
    let a = PttConfig::default();
    let b = PttConfig::default();
    let c = PttConfig {
        ptt_key: "f1".into(),
        remap_key: "f13".into(),
        source: None,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn debug_output() {
    let c = PttConfig::default();
    let d = format!("{c:?}");
    assert!(d.contains("ptt_key"));
    assert!(d.contains("remap_key"));
    assert!(d.contains("source"));
}

#[test]
fn key_code_lookup() {
    let c = PttConfig::default();
    assert_eq!(c.ptt_key_code(), Some(41)); // KEY_GRAVE
}

#[test]
fn key_code_invalid() {
    let c = PttConfig {
        ptt_key: "nonexistent".into(),
        remap_key: "f13".into(),
        source: None,
    };
    assert_eq!(c.ptt_key_code(), None);
}

#[test]
fn remap_key_name() {
    let c = PttConfig::default();
    assert_eq!(c.remap_key_name(), "f13");
}
