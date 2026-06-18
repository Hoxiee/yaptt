use anyhow::{Context, Result};
use evdev::Device;
use input_linux::{EventKind, InputId, UInputHandle};
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{info, warn};
use yaptt_daemon::*;

struct PttDaemon {
    devices: Vec<(std::path::PathBuf, String)>,
    config: PttConfig,
    active: Arc<AtomicBool>,
}

impl PttDaemon {
    fn new(config: PttConfig) -> Result<Self> {
        let devices = find_keyboard_devices();
        if devices.is_empty() {
            anyhow::bail!("No keyboard devices found");
        }
        for (path, name) in &devices {
            info!("Found keyboard: {} ({})", name, path.display());
        }
        let active = Arc::new(AtomicBool::new(true));
        Ok(Self {
            devices,
            config,
            active,
        })
    }

    async fn run(self) -> Result<()> {
        let ptt_key_code = self.config.ptt_key_code().context("Invalid PTT key")?;
        let remap_key_code = self.config.remap_key_code().context("Invalid remap key")?;
        let active = self.active.clone();
        let fade_cancel = Arc::new(AtomicBool::new(false));
        let fade_duration = self.config.fade_duration_ms;

        let loopback = match PipeWireLoopback::start("ptt-loopback") {
            Ok(lb) => {
                info!("PipeWire loopback created, node_id={}", lb.node_id);
                Some(Arc::new(Mutex::new(lb)))
            }
            Err(e) => {
                warn!("Failed to create PipeWire loopback: {}. Falling back to direct mute.", e);
                None
            }
        };

        let loopback_node: Option<String> = loopback.as_ref().map(|lb| {
            lb.lock().unwrap().node_id.to_string()
        });
        let audio_target = loopback_node.as_deref().unwrap_or("@DEFAULT_AUDIO_SOURCE@");

        let (tx, mut rx) = mpsc::unbounded_channel();

        let uinput_file = create_uinput_keyboard("ptt-virtual-keyboard")?;

        for (path, name) in &self.devices {
            let path = path.clone();
            let name = name.clone();
            let tx = tx.clone();
            let active = active.clone();
            let vk_file = uinput_file.try_clone().context("Failed to clone uinput fd")?;

            std::thread::spawn(move || {
                let mut device = match Device::open(&path) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!("Failed to open {}: {}", path.display(), e);
                        return;
                    }
                };

                if let Err(e) = device.grab() {
                    warn!("Failed to grab {}: {}", name, e);
                    return;
                }
                info!("Grabbed: {} ({})", name, path.display());

                let mut writer = vk_file;
                loop {
                    match device.fetch_events() {
                        Ok(events) => {
                            for ev in events {
                                let mut code = ev.code();
                                let raw_type = ev.event_type().0;

                                if raw_type == 1 && ev.code() == ptt_key_code && ev.value() <= 1 {
                                    if active.load(Ordering::Relaxed) {
                                        let _ = tx.send(ev.value());
                                        write_talking(ev.value() == 1);
                                        code = remap_key_code;
                                    }
                                }

                                if raw_type != 1 && raw_type != 0 {
                                    continue;
                                }

                                let kind = if raw_type == 1 {
                                    EventKind::Key
                                } else {
                                    EventKind::Synchronize
                                };

                                let input_ev = input_linux::InputEvent {
                                    time: input_linux::EventTime::new(0, 0),
                                    kind,
                                    code,
                                    value: ev.value(),
                                };

                                let ev_bytes = input_ev.into_bytes();
                                if writer.write_all(&ev_bytes).is_err() {
                                    warn!("uinput write failed on {}", name);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            warn!("evdev error on {}: {}", name, e);
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                }
            });
        }

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
            let fade_cancel = fade_cancel.clone();
            let loopback = loopback.clone();
            tokio::spawn(async move {
                let mut sigterm =
                    signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
                let mut sigint =
                    signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
                tokio::select! {
                    _ = sigterm.recv() => {},
                    _ = sigint.recv() => {},
                }
                fade_cancel.store(true, Ordering::Relaxed);
                if let Some(ref lb) = loopback {
                    let lb = lb.lock().unwrap();
                    wpctl_mute(&lb.node_id.to_string(), false);
                    drop(lb);
                } else {
                    wpctl_mute_default(false);
                }
                if active.load(Ordering::Relaxed) {
                    write_state(false);
                }
                clear_talking();
                if let Some(lb) = loopback {
                    let mut lb = lb.lock().unwrap();
                    lb.stop();
                }
                remove_pid();
                std::process::exit(0);
            });
        }

        loop {
            tokio::select! {
                Some(value) = rx.recv() => {
                    if value == 1 {
                        fade_cancel.store(true, Ordering::Relaxed);
                        wpctl_set_volume(audio_target, 1.0);
                        wpctl_mute(audio_target, false);
                    } else if value == 0 {
                        fade_cancel.store(true, Ordering::Relaxed);
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        fade_cancel.store(false, Ordering::Relaxed);
                        let cancel = fade_cancel.clone();
                        let target = audio_target.to_string();
                        std::thread::spawn(move || {
                            fade_out(&target, fade_duration, cancel);
                        });
                    }
                }
                _ = sig_rx.recv() => {
                    fade_cancel.store(true, Ordering::Relaxed);
                    if active.load(Ordering::Relaxed) {
                        wpctl_mute(audio_target, false);
                        write_state(false);
                        clear_talking();
                        active.store(false, Ordering::Relaxed);
                        info!("PTT paused");
                    } else {
                        wpctl_mute(audio_target, true);
                        write_state(true);
                        active.store(true, Ordering::Relaxed);
                        info!("PTT active");
                    }
                }
            }
        }
    }
}

fn create_uinput_keyboard(name: &str) -> Result<File> {
    let uinput_file = File::options()
        .read(true)
        .write(true)
        .open("/dev/uinput")
        .context("Failed to open /dev/uinput. Add your user to the 'input' group.")?;

    let handle = UInputHandle::new(uinput_file.try_clone().context("Failed to clone uinput fd")?);

    handle
        .set_evbit(EventKind::Key)
        .context("Failed to set EV_KEY")?;

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

    handle
        .create(&id, name.as_bytes(), 0, &[])
        .context("Failed to create uinput device")?;

    info!("Created uinput keyboard: {}", name);

    Ok(uinput_file)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    let config = load_config();
    info!(
        "Config loaded: ptt_key={}, remap_key={}, fade={}ms",
        config.ptt_key, config.remap_key, config.fade_duration_ms
    );

    let daemon = PttDaemon::new(config.clone())?;
    write_pid()?;
    write_state(false);
    clear_talking();
    info!("PTT daemon started (uinput + pw-loopback mode)");

    let _ = std::process::Command::new("notify-send")
        .args(["-a", "ptt", "-i", "microphone-sensitivity-high", "-t", "3000",
               "yaptt-daemon", "PTT daemon started. Click waybar icon to enable."])
        .output();

    daemon.run().await
}
