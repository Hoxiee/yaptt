use yaptt_daemon::{available_keys, key_code_to_name, key_name_to_code};

#[test]
fn grave_mapping() {
    assert_eq!(key_name_to_code("grave"), Some(41));
    assert_eq!(key_code_to_name(41), Some("grave".into()));
}

#[test]
fn f_keys() {
    assert_eq!(key_name_to_code("f1"), Some(59));
    assert_eq!(key_name_to_code("f13"), Some(183));
    assert_eq!(key_name_to_code("f24"), Some(194));
}

#[test]
fn letters() {
    assert_eq!(key_name_to_code("a"), Some(30));
    assert_eq!(key_name_to_code("z"), Some(44));
}

#[test]
fn modifiers() {
    assert_eq!(key_name_to_code("leftctrl"), Some(29));
    assert_eq!(key_name_to_code("leftshift"), Some(42));
    assert_eq!(key_name_to_code("leftalt"), Some(56));
    assert_eq!(key_name_to_code("leftmeta"), Some(125));
}

#[test]
fn invalid_key() {
    assert_eq!(key_name_to_code("nonexistent"), None);
}

#[test]
fn code_to_name_roundtrip() {
    for code in [41, 59, 183, 30, 29, 42, 56, 125] {
        let name = key_code_to_name(code).unwrap();
        assert_eq!(key_name_to_code(&name), Some(code));
    }
}

#[test]
fn available_keys_sorted() {
    let keys = available_keys();
    assert!(keys.len() > 50);
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[test]
fn available_keys_contains_expected() {
    let keys = available_keys();
    assert!(keys.contains(&"grave".into()));
    assert!(keys.contains(&"f1".into()));
    assert!(keys.contains(&"f13".into()));
    assert!(keys.contains(&"a".into()));
    assert!(keys.contains(&"leftctrl".into()));
}
