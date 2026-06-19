use anyhow::{Context, Result};
use clap::Parser;
use evdev::Device;
use input_linux::{EventKind, InputId, UInputHandle};
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{info, warn};
use yaptt_daemon::*;

#[derive(Parser)]
#[command(name = "yaptt-daemon", version, about = "System-wide push-to-talk daemon for Wayland")]
struct Cli {}

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
        let active = Arc::new(AtomicBool::new(false));
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
        let fade_duration = self.config.fade_duration_ms;

        let original_source_name = pactl_get_default_source()
            .context("No default audio source found. Is PipeWire running?")?;
        info!("Original default source: {}", original_source_name);

        wpctl_mute_default(false);
        info!("Mic unmuted, ready for PTT");

        let (tx, mut rx) = mpsc::unbounded_channel();
        let uinput_file = create_uinput_keyboard("ptt-virtual-keyboard")?;

        // Spawn one thread per physical keyboard to grab and forward events.
        for (path, name) in &self.devices {
            let path = path.clone();
            let name = name.clone();
            let tx = tx.clone();
            let active = active.clone();
            let vk_file = uinput_file
                .try_clone()
                .context("Failed to clone uinput fd")?;

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

                                // Intercept PTT key when active (EV_KEY=1, down/up only)
                                if raw_type == 1 && ev.code() == ptt_key_code && ev.value() <= 1 {
                                    if active.load(Ordering::Relaxed) {
                                        let _ = tx.send(ev.value());
                                        write_talking(ev.value() == 1);
                                        code = remap_key_code;
                                    }
                                }

                                // Forward only EV_KEY(1) and EV_SYN(0)
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
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            });
        }

        // SIGUSR1 handler — toggle PTT on/off
        let (sig_tx, mut sig_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut stream = signal::unix::signal(signal::unix::SignalKind::user_defined1())
                .expect("Failed to register SIGUSR1 handler");
            loop {
                stream.recv().await;
                let _ = sig_tx.send(());
            }
        });

        // SIGTERM/SIGINT handler — clean shutdown
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
                wpctl_mute_default(false);
                if active.load(Ordering::Relaxed) {
                    write_state(false);
                }
                clear_talking();
                remove_pid();
                std::process::exit(0);
            });
        }

        let fade_handle = Arc::new(std::sync::Mutex::new(None::<thread::JoinHandle<()>>));
        let fade_cancel = Arc::new(AtomicBool::new(false));
        // Volume captured when grave is pressed — used for fade and restore
        let active_vol: Arc<std::sync::Mutex<f32>> = Arc::new(std::sync::Mutex::new(1.0));

        // Main event loop
        loop {
            tokio::select! {
                Some(value) = rx.recv() => {
                    if !active.load(Ordering::Relaxed) {
                        continue;
                    }
                    if value == 1 {
                        // Grave pressed — cancel any fade, read current volume, unmute
                        fade_cancel.store(true, Ordering::Relaxed);
                        if let Some(h) = fade_handle.lock().unwrap().take() {
                            let _ = h.join();
                        }
                        let vol = wpctl_get_volume("@DEFAULT_AUDIO_SOURCE@").unwrap_or(1.0);
                        *active_vol.lock().unwrap() = vol;
                        wpctl_mute_default(false);
                        info!("PTT pressed — mic UNMUTED at {:.0}%", vol * 100.0);
                    } else if value == 0 {
                        // Grave released — fade from active_vol, then mute + restore
                        let vol = *active_vol.lock().unwrap();

                        fade_cancel.store(false, Ordering::Relaxed);
                        let cancel = fade_cancel.clone();
                        let av = active_vol.clone();
                        let fade_ms = fade_duration;

                        let handle = thread::spawn(move || {
                            let orig_vol = *av.lock().unwrap();
                            let steps = (fade_ms / 5).max(1);
                            let vol_step = orig_vol / steps as f32;
                            let mut cur_vol = orig_vol;

                            for _ in 0..steps {
                                if cancel.load(Ordering::Relaxed) {
                                    wpctl_set_volume("@DEFAULT_AUDIO_SOURCE@", orig_vol);
                                    return;
                                }
                                cur_vol = (cur_vol - vol_step).max(0.0);
                                wpctl_set_volume("@DEFAULT_AUDIO_SOURCE@", cur_vol);
                                thread::sleep(Duration::from_millis(5));
                            }

                            wpctl_mute_default(true);
                            wpctl_set_volume("@DEFAULT_AUDIO_SOURCE@", orig_vol);
                            info!("Fade complete — muted, volume restored to {:.0}%", orig_vol * 100.0);
                        });
                        *fade_handle.lock().unwrap() = Some(handle);
                        info!("PTT released — fading out ({}ms) from {:.0}%", fade_duration, vol * 100.0);
                    }
                }
                _ = sig_rx.recv() => {
                    // SIGUSR1 — toggle PTT mode (mute/unmute only, never touch volume)
                    fade_cancel.store(true, Ordering::Relaxed);
                    if let Some(h) = fade_handle.lock().unwrap().take() {
                        let _ = h.join();
                    }
                    if active.load(Ordering::Relaxed) {
                        wpctl_mute_default(false);
                        write_state(false);
                        clear_talking();
                        active.store(false, Ordering::Relaxed);
                        info!("PTT paused — mic unmuted");
                    } else {
                        wpctl_mute_default(true);
                        write_state(true);
                        active.store(true, Ordering::Relaxed);
                        info!("PTT active — mic MUTED, hold {} to talk", self.config.ptt_key);
                    }
                }
            }
        }
    }
}

/// Create a uinput virtual keyboard device.
fn create_uinput_keyboard(name: &str) -> Result<File> {
    let uinput_file = File::options()
        .read(true)
        .write(true)
        .open("/dev/uinput")
        .context("Failed to open /dev/uinput. Add your user to the 'input' group.")?;

    let handle = UInputHandle::new(
        uinput_file
            .try_clone()
            .context("Failed to clone uinput fd")?,
    );

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
    Cli::parse();
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
    info!("PTT daemon started");

    let _ = std::process::Command::new("notify-send")
        .args([
            "-a",
            "ptt",
            "-i",
            "microphone-sensitivity-high",
            "-t",
            "3000",
            "yaptt-daemon",
            "PTT daemon started. Click waybar icon to enable.",
        ])
        .output();

    daemon.run().await
}
