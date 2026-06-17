use ptt_daemon::{load_config_at, save_config_at, PttConfig};
use tempfile::TempDir;

struct ConfigTest {
    dir: TempDir,
    path: std::path::PathBuf,
}

impl ConfigTest {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        Self { dir, path }
    }
}

#[test]
fn save_and_load() {
    let t = ConfigTest::new();
    let config = PttConfig {
        ptt_key: "f5".into(),
        remap_key: "f14".into(),
        source: Some("USB mic".into()),
    };
    save_config_at(&config, &t.path).unwrap();
    let loaded = load_config_at(&t.path);
    assert_eq!(config, loaded);
}

#[test]
fn load_missing_returns_default() {
    let t = ConfigTest::new();
    let config = load_config_at(&t.path);
    assert_eq!(config, PttConfig::default());
}

#[test]
fn load_corrupt_returns_default() {
    let t = ConfigTest::new();
    std::fs::write(&t.path, "{invalid json").unwrap();
    let config = load_config_at(&t.path);
    assert_eq!(config, PttConfig::default());
}

#[test]
fn save_creates_parent_dirs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nested").join("dir").join("config.json");
    let config = PttConfig::default();
    save_config_at(&config, &path).unwrap();
    assert!(path.exists());
}

#[test]
fn roundtrip_all_fields() {
    let t = ConfigTest::new();
    let config = PttConfig {
        ptt_key: "leftctrl".into(),
        remap_key: "f20".into(),
        source: Some("Blue Yeti".into()),
    };
    save_config_at(&config, &t.path).unwrap();
    let loaded = load_config_at(&t.path);
    assert_eq!(loaded.ptt_key, "leftctrl");
    assert_eq!(loaded.remap_key, "f20");
    assert_eq!(loaded.source.as_deref(), Some("Blue Yeti"));
}

#[test]
fn overwrite_config() {
    let t = ConfigTest::new();
    let c1 = PttConfig {
        ptt_key: "f1".into(),
        remap_key: "f13".into(),
        source: None,
    };
    let c2 = PttConfig {
        ptt_key: "f2".into(),
        remap_key: "f14".into(),
        source: Some("test".into()),
    };
    save_config_at(&c1, &t.path).unwrap();
    save_config_at(&c2, &t.path).unwrap();
    let loaded = load_config_at(&t.path);
    assert_eq!(loaded, c2);
}
