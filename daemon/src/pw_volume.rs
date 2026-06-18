use libspa_sys as spa_sys;
use pipewire as pw;
use pipewire::spa;
use std::cell::{Cell, RefCell};
use std::mem::MaybeUninit;
use std::rc::Rc;

pub fn pw_init_once() {
    pw::init();
}

pub fn set_source_soft_volume(volume: f32) -> bool {
    pw_init_once();

    let mainloop = pw::main_loop::MainLoopRc::new(None).unwrap();
    let context = pw::context::ContextRc::new(&mainloop, None).unwrap();
    let core = context.connect_rc(None).unwrap();

    // Get default source ID
    let target_id = get_default_source_id();
    if target_id == 0 {
        tracing::warn!("No default source");
        return false;
    }

    // Phase 1: enumerate all globals, collect stream IDs
    let stream_ids: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    let sids = stream_ids.clone();

    {
        let registry = core.get_registry().unwrap();
        let _reg = registry.add_listener_local()
            .global(move |global| {
                let mc = global.props
                    .and_then(|p| p.get("media.class"))
                    .unwrap_or("");
                if mc.contains("Stream") || mc.contains("Input") {
                    sids.borrow_mut().push(global.id);
                }
            })
            .register();
        do_roundtrip(&mainloop, &core);
    }

    let ids = stream_ids.borrow().clone();
    if ids.is_empty() {
        tracing::warn!("No streams found");
        return false;
    }

    tracing::info!("Found {} streams, setting volume {:.0}%", ids.len(), volume * 100.0);

    // Phase 2: for each stream, enumerate again and bind+set_param
    let count = Rc::new(Cell::new(0u32));

    for &tid in &ids {
        let target_id = tid;
        let cnt = count.clone();
        let ml = mainloop.clone();
        let found = Rc::new(Cell::new(false));
        let fc = found.clone();

        {
            let registry = core.get_registry().unwrap();
            let _reg = registry.add_listener_local()
                .global(move |global| {
                    if global.id != target_id || fc.get() { return; }
                    match registry.bind(global) {
                        Ok(node) => {
                            let node: pw::node::Node = node;
                            let mut data = Vec::with_capacity(256);
                            let mut builder = spa::pod::builder::Builder::new(&mut data);

                            unsafe {
                                let mut frame = MaybeUninit::<spa_sys::spa_pod_frame>::uninit();
                                builder.push_object(&mut frame, spa_sys::SPA_PARAM_Props, 0).unwrap();
                                builder.add_prop(spa_sys::SPA_PROP_volume, 0).unwrap();
                                builder.add_float(volume).unwrap();
                                builder.pop(&mut frame.assume_init_mut());

                                let pod_ptr = &frame.assume_init().pod as *const spa_sys::spa_pod;
                                let pod = spa::pod::Pod::from_raw(pod_ptr);
                                node.set_param(spa::param::ParamType::Props, 0, pod);
                            }

                            fc.set(true);
                            cnt.set(cnt.get() + 1);
                            tracing::info!("Set volume on stream {}", target_id);
                        }
                        Err(e) => {
                            tracing::warn!("Bind failed for stream {}: {}", target_id, e);
                        }
                    }
                    ml.quit();
                })
                .register();

            do_roundtrip(&mainloop, &core);
        }
    }

    let c = count.get();
    if c > 0 {
        tracing::info!("Volume set on {} streams: {:.0}%", c, volume * 100.0);
    } else {
        tracing::warn!("Failed to set volume on any stream");
    }

    true
}

fn do_roundtrip(mainloop: &pw::main_loop::MainLoopRc, core: &pw::core::CoreRc) {
    let done = Rc::new(Cell::new(false));
    let dc = done.clone();
    let ml = mainloop.clone();
    let pending = core.sync(0).expect("sync failed");

    let _l = core.add_listener_local()
        .done(move |id, seq| {
            if id == pw::core::PW_ID_CORE && seq == pending {
                dc.set(true);
                ml.quit();
            }
        })
        .register();

    while !done.get() {
        mainloop.run();
    }
}

fn get_default_source_id() -> u32 {
    let output = std::process::Command::new("wpctl")
        .args(["inspect", "@DEFAULT_AUDIO_SOURCE@"])
        .output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("node.id") {
            if let Ok(id) = val.trim().parse::<u32>() { return id; }
        }
    }
    0
}
