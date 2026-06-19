use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct PttState {
    pub gain: AtomicU32,
    pub active: AtomicBool,
}

impl PttState {
    pub fn new() -> Self {
        Self {
            gain: AtomicU32::new(0.0f32.to_bits()),
            active: AtomicBool::new(true),
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
        let tick = Duration::from_millis(5);
        let step = if fade_duration_ms > 0 {
            1.0 / (fade_duration_ms as f32 / 5.0)
        } else {
            1.0
        };

        loop {
            thread::sleep(tick);
            let target = if state.active.load(Ordering::Relaxed) {
                1.0f32
            } else {
                0.0f32
            };
            let cur = atomic_load_f32(&state.gain);
            if (cur - target).abs() < 0.001 {
                atomic_store_f32(&state.gain, target);
                continue;
            }
            let next = if target > cur {
                (cur + step).min(1.0)
            } else {
                (cur - step).max(0.0)
            };
            atomic_store_f32(&state.gain, next);
        }
    });
}
