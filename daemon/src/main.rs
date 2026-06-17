use anyhow::{Context, Result};
use evdev::{Device, EventType, Key};
use ptt_daemon::*;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::info;

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

    async fn run(self) -> Result<()> {
        let ptt_key = self.ptt_key;
        let active = self.active.clone();
        let device = self.device;

        let (tx, mut rx) = mpsc::unbounded_channel();

        std::thread::spawn(move || {
            let mut device = device;
            loop {
                match device.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            if event.event_type() == EventType::KEY
                                && event.code() == ptt_key.code()
                            {
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

        let (sig_tx, mut sig_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut stream = signal::unix::signal(signal::unix::SignalKind::user_defined1())
                .expect("Failed to register SIGUSR1 handler");
            loop {
                stream.recv().await;
                let _ = sig_tx.send(());
            }
        });

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
                    wpctl_mute(false);
                    write_state(false);
                }
                remove_pid();
                std::process::exit(0);
            });
        }

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
                        let _ = remap_grave("grave");
                        wpctl_mute(false);
                        write_state(false);
                        active.store(false, Ordering::Relaxed);
                        info!("PTT paused");
                    } else {
                        let _ = remap_grave("f13");
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

    let config = Config::default();
    let daemon = PttDaemon::new(&config)?;
    write_pid()?;
    ptt_activate_at(Path::new(STATE_FILE))?;
    info!("PTT daemon started");

    daemon.run().await
}
