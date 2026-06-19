//! Waybar indicator for the PTT daemon.
//!
//! Outputs JSON with `text`, `class`, and `tooltip` fields.
//! States: OFF (muted icon), ON (active, waiting), TALKING (key held).

use clap::Parser;
use serde_json::json;
use std::fs;

#[derive(Parser)]
#[command(name = "yaptt-indicator", version, about = "Waybar indicator for the PTT daemon")]
struct Cli {}

const STATE_FILE: &str = "/tmp/ptt-state";
const TALKING_FILE: &str = "/tmp/ptt-talking";

fn main() {
    Cli::parse();

    let state = fs::read_to_string(STATE_FILE)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "0".to_string());

    let talking = fs::read_to_string(TALKING_FILE)
        .map(|s| s.trim() == "1")
        .unwrap_or(false);

    let (text, class, tooltip) = if state == "0" {
        (
            "\u{f131}", // nf-fa-microphone_slash
            "ptt-off",
            "Push-to-Talk: OFF\nClick to enable",
        )
    } else if talking {
        (
            "\u{f130}", // nf-fa-microphone
            "ptt-talking",
            "Push-to-Talk: ON (talking)\nRelease to mute",
        )
    } else {
        (
            "\u{f130}", // nf-fa-microphone
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
