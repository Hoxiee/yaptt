use anyhow::{Context, Result};
use evdev::{Device, EventType, Key};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::signal;
use tracing::info;

const STATE_FILE: &str = "/tmp/ptt-state";
const PID_FILE: &str = "/tmp/ptt-daemon.pid";
const KEYD_CONFIG: &str = "/etc/keyd/default.conf";

#[derive(Debug, Clone)]
struct Config {
    ptt_key: Key,
    source: Option<String>,
}

fn find_keyd_keyboard() -> Result<PathBuf> {
    let devices = evdev::enumerate();
    for (path, device) in devices {
        if let Some(name) = device.name() {
            if name.contains("keyd virtual keyboard") {
                return Ok(path);
            }
        }
    }
    anyhow::bail!("keyd virtual keyboard not found")
}

fn write_state(active: bool) {
    let _ = fs::write(STATE_FILE, if active { "1" } else { "0" });
}

fn write_pid() -> Result<()> {
    let pid = std::process::id();
    fs::write(PID_FILE, pid.to_string()).context("Failed to write PID file")?;
    Ok(())
}

fn remove_pid() {
    let _ = fs::remove_file(PID_FILE);
}

fn remap_grave(target: &str) -> Result<()> {
    let config = format!("[ids]\n\n*\n\n[main]\n\ngrave = {target}\n");
    let mut child = Command::new("sudo")
        .args(["-n", "tee", KEYD_CONFIG])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn sudo tee")?;
    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(config.as_bytes())
            .context("Failed to write config")?;
    }
    child.wait().context("Failed to wait for sudo tee")?;
    Command::new("sudo")
        .args(["-n", "keyd", "reload"])
        .output()
        .context("Failed to reload keyd")?;
    Ok(())
}

fn wpctl_mute(mute: bool) {
    let state = if mute { "1" } else { "0" };
    let _ = Command::new("wpctl")
        .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", state])
        .output();
}

struct PttDaemon {
    device: Device,
    ptt_key: Key,
    active: Arc<AtomicBool>,
}

impl PttDaemon {
    fn new(config: &Config) -> Result<Self> {
        let path = find_keyd_keyboard()?;
        let device =
            Device::open(&path).with_context(|| format!("Failed to open {}", path.display()))?;
        info!("Using: {:?}", device.name());
        let active = Arc::new(AtomicBool::new(true));
        Ok(Self {
            device,
            ptt_key: config.ptt_key,
            active,
        })
    }

    fn activate(&self) -> Result<()> {
        remap_grave("f13")?;
        wpctl_mute(true);
        write_state(true);
        info!("PTT active");
        Ok(())
    }

    fn deactivate(&self) -> Result<()> {
        remap_grave("grave")?;
        wpctl_mute(false);
        write_state(false);
        info!("PTT paused");
        Ok(())
    }

    fn toggle(&self) -> Result<()> {
        if self.active.load(Ordering::Relaxed) {
            self.deactivate()?;
            self.active.store(false, Ordering::Relaxed);
        } else {
            self.activate()?;
            self.active.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    async fn run(self) -> Result<()> {
        let ptt_key = self.ptt_key;
        let active = self.active.clone();
        let device = self.device;

        let (tx, mut rx) = mpsc::unbounded_channel();

        // evdev reader thread
        std::thread::spawn(move || {
            let mut device = device;
            loop {
                match device.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            if event.event_type() == EventType::KEY && event.code() == ptt_key.code() {
                                let _ = tx.send(event.value());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("evdev error: {e}");
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        });

        // SIGUSR1 handler
        let (sig_tx, mut sig_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut stream = signal::unix::signal(signal::unix::SignalKind::user_defined1())
                .expect("Failed to register SIGUSR1 handler");
            loop {
                stream.recv().await;
                let _ = sig_tx.send(());
            }
        });

        // SIGTERM/SIGINT cleanup
        {
            let active = self.active.clone();
            tokio::spawn(async move {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
                let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
                tokio::select! {
                    _ = sigterm.recv() => {},
                    _ = sigint.recv() => {},
                }
                if active.load(Ordering::Relaxed) {
                    let _ = remap_grave("grave");
                    wpctl_mute(false);
                    write_state(false);
                }
                remove_pid();
                std::process::exit(0);
            });
        }

        // Main event loop
        loop {
            tokio::select! {
                Some(value) = rx.recv() => {
                    if value == 1 {
                        wpctl_mute(false);
                    } else if value == 0 {
                        wpctl_mute(true);
                    }
                }
                _ = sig_rx.recv() => {
                    if active.load(Ordering::Relaxed) {
                        remap_grave("grave")?;
                        wpctl_mute(false);
                        write_state(false);
                        active.store(false, Ordering::Relaxed);
                        info!("PTT paused");
                    } else {
                        remap_grave("f13")?;
                        wpctl_mute(true);
                        write_state(true);
                        active.store(true, Ordering::Relaxed);
                        info!("PTT active");
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    let config = Config {
        ptt_key: Key::new(183),
        source: None,
    };

    let daemon = PttDaemon::new(&config)?;
    write_pid()?;
    daemon.activate()?;
    info!("PTT daemon started");

    daemon.run().await
}
