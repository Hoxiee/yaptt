use std::fs;

const STATE_FILE: &str = "/tmp/ptt-state";

fn main() {
    let state = fs::read_to_string(STATE_FILE)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0".to_string());

    let (text, class, tooltip) = if state == "1" {
        ("\u{f026c}", "ptt-active", "Push-to-Talk: ON\nClick to disable")
    } else {
        ("\u{f026d}", "ptt-inactive", "Push-to-Talk: OFF\nClick to enable")
    };

    println!(
        r#"{{"text":"{}","class":"{}","tooltip":"{}"}}"#,
        text, class, tooltip
    );
}
