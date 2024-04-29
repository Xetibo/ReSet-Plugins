use std::{collections::HashMap, time::Duration};

use dbus::{
    arg::{self, prop_cast, Append, Arg, ArgType, Get, PropMap},
    blocking::Connection,
    Error, Signature,
};

use crate::utils::{AvailableMode, DragInformation, Monitor, Offset, Scale, Size};

const BASE: &str = "org.gnome.Mutter.DisplayConfig";
const DBUS_PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const INTERFACE: &str = "org.gnome.Mutter.DisplayConfig";

// pub fn g_get_monitor_information() -> Vec<Monitor> {
//     let mut monitors = Vec::new();
//     let conn = Connection::new_session().unwrap();
//     let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
//     let res: Result<
//         (
//             u32,
//             Vec<GnomeCRTC>,
//             Vec<GnomeOutput>,
//             Vec<GnomeMode>,
//             i32,
//             i32,
//         ),
//         Error,
//     > = proxy.method_call(INTERFACE, "GetResources", ());
//     if res.is_err() {
//         println!("error on save");
//     }
//     let (serial, crtcs, outputs, modes, max_screen_width, max_screen_height) = res.unwrap();
//     let gnome_monitors = GnomeMonitors {
//         serial,
//         crtcs,
//         outputs,
//         modes,
//         max_screen_width,
//         max_screen_height,
//     };
//     dbg!(&gnome_monitors);
//     monitors
// }
//
// #[derive(Debug)]
// pub struct GnomeMonitors {
//     serial: u32,
//     crtcs: Vec<GnomeCRTC>,
//     outputs: Vec<GnomeOutput>,
//     modes: Vec<GnomeMode>,
//     max_screen_width: i32,
//     max_screen_height: i32,
// }
//
// impl<'a> Get<'a> for GnomeMonitors {
//     fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
//         let (serial, crtcs, outputs, modes, max_screen_width, max_screen_height) =
//             <(
//                 u32,
//                 Vec<GnomeCRTC>,
//                 Vec<GnomeOutput>,
//                 Vec<GnomeMode>,
//                 i32,
//                 i32,
//             )>::get(i)?;
//         Some(Self {
//             serial,
//             crtcs,
//             outputs,
//             modes,
//             max_screen_width,
//             max_screen_height,
//         })
//     }
// }
//
// impl Arg for GnomeMonitors {
//     const ARG_TYPE: arg::ArgType = ArgType::Struct;
//     fn signature() -> Signature<'static> {
//         unsafe {
//             Signature::from_slice_unchecked("ua(uxiiiiiuaua{sv})a(uxiausauauau{sv})a(uxuudu)ii\0")
//         }
//     }
// }
//
// impl GnomeMonitors {
//     // TODO: implement the conversion
//     fn to_regular_monitor(&self) -> Vec<Monitor> {
//         let mut monitors = Vec::new();
//         for output in self.outputs.iter() {
//             monitors.push(Monitor {
//                 id: output.id,
//                 name: output.name.clone(),
//                 make: "".into(),
//                 model: "".into(),
//                 serial: "".into(),
//                 refresh_rate: 0,
//                 scale: 1.0,
//                 transform: 0,
//                 vrr: false,
//                 tearing: false,
//                 offset: Offset(0, 0),
//                 size: Size(0, 0),
//                 drag_information: DragInformation::default(),
//                 available_modes: Vec::new(),
//             });
//         }
//         monitors
//     }
// }
//
// #[allow(non_snake_case)]
// #[derive(Debug)]
// pub struct GnomeCRTC {
//     id: u32,
//     winsys_id: i64,
//     x: i32,
//     y: i32,
//     width: i32,
//     height: i32,
//     mode: i32,
//     transform: u32,
//     all_transforms: Vec<u32>,
//     properties: PropMap,
// }
//
// // impl Append for GnomeCRTC {
// //     fn append_by_ref(&self, iter: &mut arg::IterAppend) {
// //         iter.append_struct(|i| {
// //             i.append(self.id);
// //             i.append(self.winsys_id);
// //             i.append(self.x);
// //             i.append(self.y);
// //             i.append(self.width);
// //             i.append(self.height);
// //             i.append(self.mode);
// //             i.append(self.transform);
// //             i.append(self.all_transforms.clone());
// //             //i.append(self.properties.clone());
// //         });
// //     }
// // }
//
// impl<'a> Get<'a> for GnomeCRTC {
//     fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
//         let (id, winsys_id, x, y, width, height, mode, transform, all_transforms, properties) =
//             <(u32, i64, i32, i32, i32, i32, i32, u32, Vec<u32>, PropMap)>::get(i)?;
//         Some(Self {
//             id,
//             winsys_id,
//             x,
//             y,
//             width,
//             height,
//             mode,
//             transform,
//             all_transforms,
//             properties,
//         })
//     }
// }
//
// impl Arg for GnomeCRTC {
//     const ARG_TYPE: arg::ArgType = ArgType::Struct;
//     fn signature() -> Signature<'static> {
//         unsafe { Signature::from_slice_unchecked("(uxiiiiiuaua{sv})\0") }
//     }
// }
//
// #[derive(Debug)]
// pub struct GnomeOutput {
//     id: u32,
//     winsys_id: i64,
//     crtc: i32,
//     all_crtcs: Vec<u32>,
//     name: String,
//     all_modes: Vec<u32>,
//     all_clones: Vec<u32>,
//     properties: PropMap,
// }
//
// impl Append for GnomeOutput {
//     fn append_by_ref(&self, iter: &mut arg::IterAppend) {
//         iter.append_struct(|i| {
//             i.append(self.id);
//             i.append(self.winsys_id);
//             i.append(self.crtc);
//             i.append(self.all_crtcs.clone());
//             i.append(self.name.clone());
//             i.append(self.all_modes.clone());
//             i.append(self.all_clones.clone());
//             //i.append(self.properties.clone());
//         });
//     }
// }
//
// impl<'a> Get<'a> for GnomeOutput {
//     fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
//         let (id, winsys_id, crtc, all_crtcs, name, all_modes, all_clones, properties) =
//             <(u32, i64, i32, Vec<u32>, String, Vec<u32>, Vec<u32>, PropMap)>::get(i)?;
//         Some(Self {
//             id,
//             winsys_id,
//             crtc,
//             all_crtcs,
//             name,
//             all_modes,
//             all_clones,
//             properties,
//         })
//     }
// }
//
// impl Arg for GnomeOutput {
//     const ARG_TYPE: arg::ArgType = ArgType::Struct;
//     fn signature() -> Signature<'static> {
//         unsafe { Signature::from_slice_unchecked("(uxiausauauau{sv})\0") }
//     }
// }
//
// #[derive(Debug)]
// pub struct GnomeMode {
//     id: u32,
//     winsys_id: i64,
//     width: u32,
//     height: u32,
//     frequency: f64,
//     flags: u32,
// }
//
// impl Append for GnomeMode {
//     fn append_by_ref(&self, iter: &mut arg::IterAppend) {
//         iter.append_struct(|i| {
//             i.append(self.id);
//             i.append(self.winsys_id);
//             i.append(self.width);
//             i.append(self.height);
//             i.append(self.frequency);
//             i.append(self.flags);
//         });
//     }
// }
//
// impl<'a> Get<'a> for GnomeMode {
//     fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
//         let (id, winsys_id, width, height, frequency, flags) =
//             <(u32, i64, u32, u32, f64, u32)>::get(i)?;
//         Some(Self {
//             id,
//             winsys_id,
//             width,
//             height,
//             frequency,
//             flags,
//         })
//     }
// }
//
// impl Arg for GnomeMode {
//     const ARG_TYPE: arg::ArgType = ArgType::Struct;
//     fn signature() -> Signature<'static> {
//         unsafe { Signature::from_slice_unchecked("(uxuudu)\0") }
//     }
// }

///

pub fn g_get_monitor_information() -> Vec<Monitor> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(u32, Vec<GnomeMonitor>, Vec<GnomeLogicalMonitor>, PropMap), Error> =
        proxy.method_call(INTERFACE, "GetCurrentState", ());
    if res.is_err() {
        dbg!(&res);
        println!("error on save");
    }
    let (serial, monitors, logical_monitors, properties) = res.unwrap();
    let gnome_monitors = GnomeMonitorConfig {
        serial,
        monitors,
        logical_monitors,
        properties,
    };
    dbg!(&gnome_monitors);
    gnome_monitors.to_regular_monitor()
}

pub fn g_apply_monitor_config(monitors: &Vec<Monitor>) {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(), Error> = proxy.method_call(
        INTERFACE,
        "ApplyMonitorConfig",
        (GnomeMonitorConfig::from_regular_monitor(monitors),),
    );
    if res.is_err() {
        dbg!(&res);
        println!("error on save");
    }
    println!("ok");
}

#[derive(Debug)]
pub struct GnomeMonitorConfig {
    serial: u32,
    monitors: Vec<GnomeMonitor>,
    logical_monitors: Vec<GnomeLogicalMonitor>,
    properties: PropMap,
}

impl GnomeMonitorConfig {
    // TODO: implement the conversion
    fn to_regular_monitor(&self) -> Vec<Monitor> {
        let mut monitors = Vec::new();
        for (monitor, logical_monitor) in self.monitors.iter().zip(self.logical_monitors.iter()) {
            let mut hash_modes: HashMap<usize, AvailableMode> = HashMap::new();
            let mut modes = Vec::new();
            let mut current_mode: Option<&GnomeMode> = None;
            for mode in monitor.modes.iter() {
                let flag_opt: Option<&bool> = prop_cast(&mode.properties, "is-current");
                if let Some(flag) = flag_opt {
                    if *flag {
                        current_mode = Some(mode);
                    }
                }
                if let Some(saved_mode) = hash_modes.get_mut(&hash_modes.len()) {
                    saved_mode
                        .refresh_rates
                        .push(mode.refresh_rate.round() as u32);
                } else {
                    hash_modes.insert(
                        hash_modes.len() + 1,
                        AvailableMode {
                            id: mode.id.clone(),
                            size: Size(mode.width, mode.height),
                            refresh_rates: vec![mode.refresh_rate.round() as u32],
                        },
                    );
                }
            }
            for (_, mode) in hash_modes {
                modes.push(mode);
            }
            if current_mode.is_none() {
                return Vec::new();
            }
            let current_mode = current_mode.unwrap();
            let mut vrr = false;
            let refresh_rate_opt: Option<&String> =
                prop_cast(&current_mode.properties, "refresh-rate-mode");
            if let Some(refresh_rate_mode) = refresh_rate_opt {
                if refresh_rate_mode == "variable" {
                    vrr = true;
                }
            }

            monitors.push(Monitor {
                id: self.serial,
                name: monitor.name.connector.clone(),
                make: monitor.name.vendor.clone(),
                model: monitor.name.product.clone(),
                serial: monitor.name.serial.clone(),
                refresh_rate: current_mode.refresh_rate.round() as u32,
                scale: logical_monitor.scale,
                transform: logical_monitor.transform,
                vrr,
                // gnome doesn't support this as of now
                tearing: false,
                offset: Offset(logical_monitor.x, logical_monitor.y),
                size: Size(current_mode.width, current_mode.height),
                mode: current_mode.id.clone(),
                drag_information: DragInformation::default(),
                available_modes: modes,
            });
        }
        monitors
    }

    pub fn from_regular_monitor(
        monitors: &Vec<Monitor>,
    ) -> (u32, u32, Vec<GnomeLogicalMonitorSend>, PropMap) {
        let mut g_logical_monitors = Vec::new();
        for monitor in monitors {
            g_logical_monitors.push(GnomeLogicalMonitorSend {
                x: monitor.offset.0,
                y: monitor.offset.1,
                scale: monitor.scale,
                transform: monitor.transform,
                // TODO:
                primary: false,
                // TODO:
                monitors: vec![(monitor.name.clone(), monitor.mode.clone(), PropMap::new())],
            });
        }
        // TODO: make method dynamic -> 1 is temporary -> e.g. apply
        (0, 1, g_logical_monitors, PropMap::new())
    }
}

#[derive(Debug)]
pub struct GnomeMonitor {
    name: GnomeName,
    modes: Vec<GnomeMode>,
    properties: PropMap,
}

impl<'a> Get<'a> for GnomeMonitor {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (name, modes, properties) = <(GnomeName, Vec<GnomeMode>, PropMap)>::get(i)?;
        Some(Self {
            name,
            modes,
            properties,
        })
    }
}

impl Arg for GnomeMonitor {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("((ssss)a(siiddada{sv})a{sv})\0") }
    }
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct GnomeMode {
    id: String,
    width: i32,
    height: i32,
    refresh_rate: f64,
    scale: f64,
    // technically gnome specifies supported scales
    // however, as long as the width and height resolve to integers, the scaling should work
    supported_scales: Vec<f64>,
    properties: PropMap,
}

impl<'a> Get<'a> for GnomeMode {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (id, width, height, refresh_rate, scale, supported_scales, properties) =
            <(String, i32, i32, f64, f64, Vec<f64>, PropMap)>::get(i)?;
        Some(Self {
            id,
            width,
            height,
            refresh_rate,
            scale,
            supported_scales,
            properties,
        })
    }
}

impl Arg for GnomeMode {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(siiddada{sv})\0") }
    }
}

#[derive(Debug)]
pub struct GnomeLogicalMonitor {
    x: i32,
    y: i32,
    scale: f64,
    transform: u32,
    primary: bool,
    monitors: Vec<(String, String, String, String)>,
    properties: PropMap,
}

impl<'a> Get<'a> for GnomeLogicalMonitor {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (x, y, scale, transform, primary, monitors, properties) = <(
            i32,
            i32,
            f64,
            u32,
            bool,
            Vec<(String, String, String, String)>,
            PropMap,
        )>::get(i)?;
        Some(Self {
            x,
            y,
            scale,
            transform,
            primary,
            monitors,
            properties,
        })
    }
}

impl Arg for GnomeLogicalMonitor {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(iiduba(ssss)a{sv})\0") }
    }
}

#[derive(Debug)]
pub struct GnomeLogicalMonitorSend {
    x: i32,
    y: i32,
    scale: f64,
    transform: u32,
    primary: bool,
    monitors: Vec<(String, String, PropMap)>,
}

impl<'a> Get<'a> for GnomeLogicalMonitorSend {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (x, y, scale, transform, primary, monitors) =
            <(i32, i32, f64, u32, bool, Vec<(String, String, PropMap)>)>::get(i)?;
        Some(Self {
            x,
            y,
            scale,
            transform,
            primary,
            monitors,
        })
    }
}

impl Arg for GnomeLogicalMonitorSend {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(iiduba(ssa{sv}))\0") }
    }
}

impl Append for GnomeLogicalMonitorSend {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        let monitor = self.monitors.first().unwrap();
        iter.append_struct(|i| {
            i.append(self.x);
            i.append(self.y);
            i.append(self.scale);
            i.append(self.transform);
            i.append(self.primary);
            i.append(vec![(monitor.0.clone(), monitor.1.clone(), PropMap::new())]);
        });
    }
}

#[derive(Debug)]
pub struct GnomeName {
    connector: String,
    vendor: String,
    product: String,
    serial: String,
}

impl<'a> Get<'a> for GnomeName {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (connector, vendor, product, serial) = <(String, String, String, String)>::get(i)?;
        Some(Self {
            connector,
            vendor,
            product,
            serial,
        })
    }
}

impl Arg for GnomeName {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ssss)\0") }
    }
}
