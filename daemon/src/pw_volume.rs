use anyhow::Context;
use pipewire as pw;
use pipewire::properties::properties;
use pipewire::spa;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct PttState {
    pub gain: AtomicU32,
    pub active: AtomicBool,
    pub muted: AtomicBool,
}

impl PttState {
    pub fn new() -> Self {
        Self {
            gain: AtomicU32::new(1.0f32.to_bits()),
            active: AtomicBool::new(false),
            muted: AtomicBool::new(true),
        }
    }
}

pub fn atomic_load_f32(a: &AtomicU32) -> f32 {
    f32::from_bits(a.load(Ordering::Relaxed))
}

pub fn atomic_store_f32(a: &AtomicU32, v: f32) {
    a.store(v.to_bits(), Ordering::Relaxed);
}

pub fn start_fade_thread(state: Arc<PttState>, fade_duration_ms: u64) {
    thread::spawn(move || {
        let tick = Duration::from_millis(10);
        let step = 10.0 / fade_duration_ms as f32;

        loop {
            thread::sleep(tick);
            let target = if state.active.load(Ordering::Relaxed) {
                1.0f32
            } else {
                0.0f32
            };
            let cur = atomic_load_f32(&state.gain);
            if (cur - target).abs() < 0.005 {
                atomic_store_f32(&state.gain, target);
                state.muted.store(target == 0.0, Ordering::Relaxed);
                continue;
            }
            if target > cur {
                state.muted.store(false, Ordering::Relaxed);
                atomic_store_f32(&state.gain, (cur + step).min(1.0));
            } else {
                atomic_store_f32(&state.gain, (cur - step).max(0.0));
            }
        }
    });
}

pub fn create_ptt_stream(state: Arc<PttState>) -> anyhow::Result<()> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .context("Failed to create PipeWire main loop")?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .context("Failed to create PipeWire context")?;
    let core = context.connect_rc(None)
        .context("Failed to connect to PipeWire")?;

    let stream = pw::stream::StreamRc::new(
        core.clone(),
        "ptt-volume-filter",
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Source",
            *pw::keys::MEDIA_ROLE => "Communication",
            *pw::keys::NODE_NAME => "PTT Volume Filter",
            *pw::keys::NODE_DESCRIPTION => "PTT Push-to-Talk Volume Control",
        },
    )
    .context("Failed to create stream")?;

    stream
        .connect(
            spa::utils::Direction::Input,
            None,
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS,
            &mut [],
        )
        .context("Failed to connect stream")?;

    let _listener = stream
        .add_local_listener_with_user_data(UserData { state })
        .process(|_stream, data| {
            let Some(mut buf) = _stream.dequeue_buffer() else {
                return;
            };
            let datas = buf.datas_mut();
            let Some(d) = datas.get_mut(0) else {
                return;
            };

            let gain = atomic_load_f32(&data.state.gain);

            let Some(data_slice) = d.data() else {
                return;
            };
            let byte_len = data_slice.len();
            let ptr = data_slice.as_mut_ptr() as *mut f32;

            if data.state.muted.load(Ordering::Relaxed) {
                unsafe {
                    std::ptr::write_bytes(ptr, 0, byte_len);
                }
                return;
            }

            let samples = unsafe { std::slice::from_raw_parts_mut(ptr, byte_len / 4) };
            for s in samples.iter_mut() {
                *s *= gain;
            }
        })
        .register()
        .context("Failed to register stream listener")?;

    mainloop.run();

    Ok(())
}

struct UserData {
    state: Arc<PttState>,
}
