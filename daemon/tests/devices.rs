use yaptt_daemon::find_keyboard_devices;

#[test]
fn find_keyboard_devices_does_not_include_mice() {
    let devices = find_keyboard_devices();
    for (path, name) in &devices {
        let lower = name.to_lowercase();
        assert!(
            !lower.contains("mouse") && !lower.contains("pointer") && !lower.contains("touchpad"),
            "Device should not be a mouse/touchpad: {} ({})",
            name,
            path.display()
        );
    }
}

#[test]
fn find_keyboard_devices_skips_system_devices() {
    let devices = find_keyboard_devices();
    for (path, name) in &devices {
        let lower = name.to_lowercase();
        assert!(
            !lower.contains("power button")
                && !lower.contains("video bus")
                && !lower.contains("pc speaker")
                && !lower.contains("hda nvidia")
                && !lower.contains("hd-audio")
                && !lower.contains("vicinae")
                && !lower.contains("system control")
                && !lower.contains("consumer control")
                && !lower.contains("audio"),
            "Device should be skipped: {} ({})",
            name,
            path.display()
        );
    }
}

#[test]
fn find_keyboard_devices_only_kbd_not_mouse() {
    let proc = std::fs::read_to_string("/proc/bus/input/devices").unwrap_or_default();
    let devices = find_keyboard_devices();

    for (path, _name) in &devices {
        let event_name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let mut blocks: Vec<(String, String)> = Vec::new();
        let mut cur_name = String::new();
        let mut cur_handlers = String::new();

        for line in proc.lines() {
            if line.starts_with('N') && line.contains("Name=") {
                cur_name = line
                    .split_once("Name=")
                    .map(|(_, v)| v.trim_matches('"').to_string())
                    .unwrap_or_default();
            } else if line.starts_with('H') && line.contains("Handlers=") {
                cur_handlers = line
                    .split_once("Handlers=")
                    .map(|(_, v)| v.trim().to_string())
                    .unwrap_or_default();
            } else if line.starts_with('I') && !line.contains("ID_") {
                blocks.push((cur_name.clone(), cur_handlers.clone()));
                cur_name.clear();
                cur_handlers.clear();
            }
        }

        let matching: Vec<_> = blocks
            .iter()
            .filter(|(_, h)| h.split_whitespace().any(|x| x == event_name.as_str()))
            .collect();

        assert!(!matching.is_empty(), "event{} not found in /proc", &event_name[5..]);

        for (_name, handlers) in &matching {
            assert!(
                handlers.contains("kbd"),
                "event{} should have kbd handler, got: {}",
                &event_name[5..],
                handlers
            );
            assert!(
                !handlers.contains("mouse"),
                "event{} should NOT have mouse handler, got: {}",
                &event_name[5..],
                handlers
            );
        }
    }
}

#[test]
fn find_keyboard_devices_all_paths_exist() {
    let devices = find_keyboard_devices();
    for (path, name) in &devices {
        assert!(
            path.exists(),
            "Device path does not exist: {} ({})",
            name,
            path.display()
        );
    }
}

#[test]
fn find_keyboard_devices_deduplicates() {
    let devices = find_keyboard_devices();
    let mut paths: Vec<_> = devices.iter().map(|(p, _)| p).collect();
    let original_len = paths.len();
    paths.sort();
    paths.dedup();
    assert_eq!(
        original_len,
        paths.len(),
        "Duplicate device paths found in keyboard devices"
    );
}
