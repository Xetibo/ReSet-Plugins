use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use xrandr::{Crtc, Mode, ScreenResources, XHandle, XId};

use crate::utils::{AvailableMode, Monitor, MonitorFeatures, Offset, Size, XHandleWrapper};

pub fn x11_get_monitor_information() -> Vec<Monitor> {
    let maybe_x11_conn = XHandle::open();
    if maybe_x11_conn.is_err() {
        // TODO: print error
        return Vec::new();
    }
    let mut handle = maybe_x11_conn.unwrap();
    let res = ScreenResources::new(&mut handle);
    if res.is_err() {
        // TODO: print error
        return Vec::new();
    }
    let res = res.unwrap();
    let empty_mode = Mode {
        xid: 0,
        width: 0,
        height: 0,
        dot_clock: 0,
        hsync_tart: 0,
        hsync_end: 0,
        htotal: 0,
        hskew: 0,
        vsync_start: 0,
        vsync_end: 0,
        vtotal: 0,
        name: "".into(),
        flags: 0,
        rate: 0.0,
    };

    // TODO:
    let mut monitors = Vec::new();
    let primary = 0;
    let mut hash_modes: HashMap<Size, (String, HashSet<u32>, Vec<f64>)> = HashMap::new();
    let mut modes = Vec::new();
    let mut current_mode: Option<&Mode> = None;
    for mode in res.modes.iter() {
        if mode.xid == primary {
            current_mode = Some(mode);
        }
        if let Some(saved_mode) = hash_modes.get_mut(&Size(mode.width as i32, mode.height as i32)) {
            saved_mode.1.insert(mode.rate.round() as u32);
        } else {
            let mut refresh_rates = HashSet::new();
            refresh_rates.insert(mode.rate.round() as u32);
            hash_modes.insert(
                Size(mode.width as i32, mode.height as i32),
                (mode.xid.to_string(), refresh_rates, Vec::new()),
            );
        }
    }
    for (size, (id, refresh_rates, supported_scales)) in hash_modes {
        let mut refresh_rates: Vec<u32> = refresh_rates.into_iter().collect();
        refresh_rates.sort_unstable_by(|a, b| {
            if a > b {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });
        modes.push(AvailableMode {
            id,
            size,
            refresh_rates,
            supported_scales,
        });
    }
    modes.sort_unstable_by(|a, b| {
        if a.size > b.size {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });
    let outputs = res.outputs(&mut handle);
    if outputs.is_err() {
        // TODO: print error
        return Vec::new();
    }
    let outputs = outputs.unwrap();
    for output in outputs {
        let first_mode = res.modes.first().unwrap().xid;
        let mut available_modes = Vec::new();
        for mode in output.modes {
            for other_mode in modes.iter() {
                if let Some(current) = output.current_mode {
                    // if current == mode &&  
                }
                if other_mode.id.parse::<u64>().unwrap() == mode {
                    available_modes.push(other_mode.clone());
                }
            }
        }
        if current_mode.is_none() {
            current_mode = Some(&empty_mode);
        }
        let current_mode = current_mode.unwrap();
        let crtc = Crtc::from_xid(&mut handle, output.crtc.unwrap()).unwrap();
        monitors.push(Monitor {
            id: output.xid as u32,
            enabled: output.current_mode.is_some(),
            name: output.name,
            make: "".into(),
            model: "".into(),
            serial: "".into(),
            refresh_rate: current_mode.rate.round() as u32,
            //TODO:
            scale: 1.0,
            //TODO:
            transform: 0,
            vrr: false,
            primary: output.is_primary,
            offset: Offset(crtc.x, crtc.y),
            size: Size(current_mode.width as i32, current_mode.height as i32),
            mode: output.current_mode.unwrap_or(0).to_string(),
            available_modes,
            features: MonitorFeatures {
                vrr: false,
                primary: true,
                fractional_scaling: true,
                hdr: false,
            },
            ..Default::default()
        });
    }
    monitors
}

// pub fn x11_get_monitor_information(conn: Option<Arc<(RustConnection, Window)>>) -> Vec<Monitor> {
//     if let Some(conn) = conn {
//         let monitors =
//             x11rb::protocol::randr::get_monitors::<RustConnection>(&conn.0, conn.1, false);
//         for screen in monitors.iter() {
//             dbg!(screen);
//         }
//     }
//     Vec::new()
// }
