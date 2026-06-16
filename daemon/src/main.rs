use anyhow::{Context, Result};
use evdev::{Device, EventType, InputEvent, Key};
use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::unix::AsyncFd;
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

fn handle_event(event: InputEvent, ptt_key: Key) {
    if event.event_type() != EventType::KEY {
        return;
    }

    let key = Key::new(event.code());
    if key == ptt_key {
        let pressed = event.value() == 1;
        let muted = !pressed;
        wpctl_mute(muted);
    }
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

    async fn run(mut self) -> Result<()> {
        let (signal_tx, mut signal_rx) = tokio::sync::mpsc::channel::<()>(1);

        // Handle SIGUSR1 for toggle
        {
            let signal_tx = signal_tx.clone();
            tokio::spawn(async move {
                let mut stream = signal::unix::signal(signal::unix::SignalKind::user_defined1())
                    .expect("Failed to register SIGUSR1 handler");
                loop {
                    stream.recv().await;
                    if signal_tx.send(()).await.is_err() {
                        break;
                    }
                }
            });
        }

        // Handle SIGTERM/SIGINT for cleanup
        {
            let active = self.active.clone();
            tokio::spawn(async move {
                let mut sigterm =
                    signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
                let mut sigint =
                    signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
                tokio::select! {
                    _ = sigterm.recv() => {},
                    _ = sigint.recv() => {},
                }
                if active.load(Ordering::Relaxed) {
                    let _ = remap_grave("grave");
                    let _ = Command::new("wpctl")
                        .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "0"])
                        .output();
                    write_state(false);
                }
                remove_pid();
                std::process::exit(0);
            });
        }

        // Main event loop
        let fd = self.device.as_raw_fd();
        let async_fd = AsyncFd::new(fd).context("Failed to create AsyncFd")?;
        let ptt_key = self.ptt_key;

        loop {
            tokio::select! {
                result = async_fd.readable() => {
                    match result {
                        Ok(mut guard) => {
                            if guard.try_io(|_| {
                                let events: Vec<_> = self.device.fetch_events().unwrap().collect();
                                for event in events {
                                    handle_event(event, ptt_key);
                                }
                                Ok::<(), std::io::Error>(())
                            }).is_err() {
                                continue;
                            }
                        }
                        Err(_) => continue,
                    }
                }
                _ = signal_rx.recv() => {
                    self.toggle()?;
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
        ptt_key: Key::new(183), // KEY_F13
        source: None,
    };

    let daemon = PttDaemon::new(&config)?;

    write_pid()?;
    daemon.activate()?;

    info!("PTT daemon started");

    daemon.run().await
}
