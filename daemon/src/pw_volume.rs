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

    let mainloop = match pw::main_loop::MainLoopRc::new(None) {
        Ok(ml) => ml,
        Err(e) => { tracing::warn!("PipeWire main loop: {}", e); return false; }
    };
    let context = match pw::context::ContextRc::new(&mainloop, None) {
        Ok(ctx) => ctx,
        Err(e) => { tracing::warn!("PipeWire context: {}", e); return false; }
    };
    let core = match context.connect_rc(None) {
        Ok(c) => c,
        Err(e) => { tracing::warn!("PipeWire connect: {}", e); return false; }
    };
    let registry = match core.get_registry() {
        Ok(r) => r,
        Err(e) => { tracing::warn!("PipeWire registry: {}", e); return false; }
    };

    // Phase 1: collect stream IDs
    let stream_ids: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    let sids = stream_ids.clone();
    let ml1 = mainloop.clone();

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
    let ids = stream_ids.borrow().clone();
    drop(_reg);

    if ids.is_empty() {
        tracing::warn!("No streams found");
        return false;
    }

    tracing::info!("Found {} streams, setting volume {:.0}%", ids.len(), volume * 100.0);

    // Phase 2: bind each stream and set volume
    let count = Rc::new(Cell::new(0u32));

    for target_id in &ids {
        let tid = *target_id;
        let found: Rc<Cell<Option<u32>>> = Rc::new(Cell::new(None));
        let fc = found.clone();
        let ml2 = mainloop.clone();

        let _reg2 = registry.add_listener_local()
            .global(move |global| {
                if global.id == tid {
                    fc.set(Some(global.id));
                    ml2.quit();
                }
            })
            .register();

        do_roundtrip(&mainloop, &core);

        if found.get().is_none() {
            drop(_reg2);
            continue;
        }

        // We found the global, now we need to bind it.
        // Problem: we can't access the GlobalObject from here.
        // Solution: do a third pass that binds directly

        let bound_ok = Rc::new(Cell::new(false));
        let bo = bound_ok.clone();
        let ml3 = mainloop.clone();

        let _reg3 = registry.add_listener_local()
            .global(move |global| {
                if global.id == tid {
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
                                builder.pop(&mut frame.assume_init());
                            }

                            let pod = unsafe { spa::pod::Pod::from_raw(builder.deref()) };
                            node.set_param(spa::param::ParamType::Props, 0, &pod);
                            bo.set(true);
                        }
                        Err(e) => {
                            tracing::warn!("Bind failed for {}: {}", tid, e);
                        }
                    }
                    ml3.quit();
                }
            })
            .register();

        do_roundtrip(&mainloop, &core);

        if bound_ok.get() {
            count.set(count.get() + 1);
            tracing::info!("Set volume on stream {}", tid);
        }

        drop(_reg3);
    }

    let c = count.get();
    if c > 0 {
        tracing::info!("Volume set on {} streams: {:.0}%", c, volume * 100.0);
        true
    } else {
        tracing::warn!("Failed to set volume on any stream");
        false
    }
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
