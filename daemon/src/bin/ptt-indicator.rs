use serde_json::json;
use std::fs;

const STATE_FILE: &str = "/tmp/ptt-state";
const TALKING_FILE: &str = "/tmp/ptt-talking";

fn main() {
    let state = fs::read_to_string(STATE_FILE)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0".to_string());

    let talking = fs::read_to_string(TALKING_FILE)
        .map(|s| s.trim() == "1")
        .unwrap_or(false);

    let (text, class, tooltip) = if state == "0" {
        (
            "\u{f026d}", // nf-md-microphone_off
            "ptt-off",
            "Push-to-Talk: OFF\nClick to enable",
        )
    } else if talking {
        (
            "\u{f1314}", // nf-md-microphone_message
            "ptt-talking",
            "Push-to-Talk: ON (talking)\nRelease to mute",
        )
    } else {
        (
            "\u{f026c}", // nf-md-microphone
            "ptt-on",
            "Push-to-Talk: ON\nHold grave to talk\nClick to disable",
        )
    };

    let obj = json!({
        "text": text,
        "class": class,
        "tooltip": tooltip,
    });
    println!("{obj}");
}
