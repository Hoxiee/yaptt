use std::fs::File;
use input_linux::{EventKind, InputId, UInputHandle};

fn skip_if_no_uinput() {
    if File::options().read(true).write(true).open("/dev/uinput").is_err() {
        eprintln!("Skipping: /dev/uinput not accessible (need input group or root)");
        std::process::exit(0);
    }
}

fn create_test_uinput(name: &str) -> Result<File, Box<dyn std::error::Error>> {
    let uinput_file = File::options()
        .read(true)
        .write(true)
        .open("/dev/uinput")?;

    let handle = UInputHandle::new(uinput_file.try_clone()?);

    handle.set_evbit(EventKind::Key)?;

    for code in 0u16..=0x2ff {
        if let Ok(key) = input_linux::Key::from_code(code) {
            let _ = handle.set_keybit(key);
        }
    }

    let id = InputId {
        bustype: 0x03,
        vendor: 0,
        product: 0,
        version: 0,
    };

    handle.create(&id, name.as_bytes(), 0, &[])?;

    Ok(uinput_file)
}

#[test]
fn uinput_keyboard_creation() {
    skip_if_no_uinput();
    let result = create_test_uinput("test-ptt-keyboard");
    assert!(result.is_ok(), "Failed to create uinput device: {:?}", result.err());
}

#[test]
fn uinput_keyboard_can_write_event() {
    skip_if_no_uinput();
    let file = create_test_uinput("test-ptt-write").unwrap();
    let handle = UInputHandle::new(file);

    let ev = input_linux::InputEvent {
        time: input_linux::EventTime::new(0, 0),
        kind: EventKind::Key,
        code: 30,
        value: 1,
    };

    let result = handle.write(&[unsafe { std::mem::transmute(ev) }]);
    assert!(result.is_ok(), "Failed to write event: {:?}", result.err());
}

#[test]
fn uinput_keyboard_fd_clone() {
    skip_if_no_uinput();
    let file = create_test_uinput("test-ptt-clone").unwrap();
    let file2 = file.try_clone().unwrap();

    let ev = input_linux::InputEvent {
        time: input_linux::EventTime::new(0, 0),
        kind: EventKind::Key,
        code: 30,
        value: 1,
    };

    let h1 = UInputHandle::new(file);
    let h2 = UInputHandle::new(file2);

    let raw_ev: input_linux::sys::input_event = unsafe { std::mem::transmute(ev) };
    assert!(h1.write(&[raw_ev]).is_ok());
    assert!(h2.write(&[raw_ev]).is_ok());
}

#[test]
fn uinput_syn_report_only() {
    skip_if_no_uinput();
    let file = create_test_uinput("test-ptt-syn").unwrap();
    let handle = UInputHandle::new(file);

    let key_ev = input_linux::InputEvent {
        time: input_linux::EventTime::new(0, 0),
        kind: EventKind::Key,
        code: 30,
        value: 1,
    };
    let syn_ev = input_linux::InputEvent {
        time: input_linux::EventTime::new(0, 0),
        kind: EventKind::Synchronize,
        code: 0,
        value: 0,
    };

    let raw_key: input_linux::sys::input_event = unsafe { std::mem::transmute(key_ev) };
    let raw_syn: input_linux::sys::input_event = unsafe { std::mem::transmute(syn_ev) };

    let result = handle.write(&[raw_key, raw_syn]);
    assert!(result.is_ok(), "Failed to write key+syn: {:?}", result.err());
}
