use std::time::Duration;

use dbus::{
    arg::{self, Append, Arg, ArgType, Get, PropMap},
    blocking::Connection,
    Error, Signature,
};

use crate::utils::{DragInformation, Monitor, Offset, Scale, Size};

const BASE: &str = "org.gnome.Mutter.DisplayConfig";
const DBUS_PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const INTERFACE: &str = "org.gnome.Mutter.DisplayConfig";

pub fn g_get_monitor_information() -> Vec<Monitor> {
    let mut monitors = Vec::new();
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<
        (
            u32,
            Vec<GnomeCRTC>,
            Vec<GnomeOutput>,
            Vec<GnomeMode>,
            i32,
            i32,
        ),
        Error,
    > = proxy.method_call(INTERFACE, "GetResources", ());
    if res.is_err() {
        println!("error on save");
    }
    let (serial, crtcs, outputs, modes, max_screen_width, max_screen_height) = res.unwrap();
    let gnome_monitors = GnomeMonitors {
        serial,
        crtcs,
        outputs,
        modes,
        max_screen_width,
        max_screen_height,
    };
    dbg!(&gnome_monitors);
    monitors
}

#[derive(Debug)]
pub struct GnomeMonitors {
    serial: u32,
    crtcs: Vec<GnomeCRTC>,
    outputs: Vec<GnomeOutput>,
    modes: Vec<GnomeMode>,
    max_screen_width: i32,
    max_screen_height: i32,
}

impl<'a> Get<'a> for GnomeMonitors {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (serial, crtcs, outputs, modes, max_screen_width, max_screen_height) =
            <(
                u32,
                Vec<GnomeCRTC>,
                Vec<GnomeOutput>,
                Vec<GnomeMode>,
                i32,
                i32,
            )>::get(i)?;
        Some(Self {
            serial,
            crtcs,
            outputs,
            modes,
            max_screen_width,
            max_screen_height,
        })
    }
}

impl Arg for GnomeMonitors {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe {
            Signature::from_slice_unchecked("ua(uxiiiiiuaua{sv})a(uxiausauauau{sv})a(uxuudu)ii\0")
        }
    }
}

impl GnomeMonitors {
    // TODO: implement the conversion
    fn to_regular_monitor(&self) -> Vec<Monitor> {
        let mut monitors = Vec::new();
        for output in self.outputs.iter() {
            monitors.push(Monitor {
                id: output.id,
                name: output.name.clone(),
                make: "".into(),
                model: "".into(),
                serial: "".into(),
                refresh_rate: 0,
                scale: Scale(0, 0),
                transform: 0,
                vrr: false,
                tearing: false,
                offset: Offset(0, 0),
                size: Size(0, 0),
                drag_information: DragInformation::default(),
                available_modes: Vec::new(),
            });
        }
        monitors
    }
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct GnomeCRTC {
    id: u32,
    winsys_id: i64,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    mode: i32,
    transform: u32,
    all_transforms: Vec<u32>,
    properties: PropMap,
}

// impl Append for GnomeCRTC {
//     fn append_by_ref(&self, iter: &mut arg::IterAppend) {
//         iter.append_struct(|i| {
//             i.append(self.id);
//             i.append(self.winsys_id);
//             i.append(self.x);
//             i.append(self.y);
//             i.append(self.width);
//             i.append(self.height);
//             i.append(self.mode);
//             i.append(self.transform);
//             i.append(self.all_transforms.clone());
//             //i.append(self.properties.clone());
//         });
//     }
// }

impl<'a> Get<'a> for GnomeCRTC {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (id, winsys_id, x, y, width, height, mode, transform, all_transforms, properties) =
            <(u32, i64, i32, i32, i32, i32, i32, u32, Vec<u32>, PropMap)>::get(i)?;
        Some(Self {
            id,
            winsys_id,
            x,
            y,
            width,
            height,
            mode,
            transform,
            all_transforms,
            properties,
        })
    }
}

impl Arg for GnomeCRTC {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(uxiiiiiuaua{sv})\0") }
    }
}

#[derive(Debug)]
pub struct GnomeOutput {
    id: u32,
    winsys_id: i64,
    crtc: i32,
    all_crtcs: Vec<u32>,
    name: String,
    all_modes: Vec<u32>,
    all_clones: Vec<u32>,
    properties: PropMap,
}

impl Append for GnomeOutput {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.id);
            i.append(self.winsys_id);
            i.append(self.crtc);
            i.append(self.all_crtcs.clone());
            i.append(self.name.clone());
            i.append(self.all_modes.clone());
            i.append(self.all_clones.clone());
            //i.append(self.properties.clone());
        });
    }
}

impl<'a> Get<'a> for GnomeOutput {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (id, winsys_id, crtc, all_crtcs, name, all_modes, all_clones, properties) =
            <(u32, i64, i32, Vec<u32>, String, Vec<u32>, Vec<u32>, PropMap)>::get(i)?;
        Some(Self {
            id,
            winsys_id,
            crtc,
            all_crtcs,
            name,
            all_modes,
            all_clones,
            properties,
        })
    }
}

impl Arg for GnomeOutput {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(uxiausauauau{sv})\0") }
    }
}

#[derive(Debug)]
pub struct GnomeMode {
    id: u32,
    winsys_id: i64,
    width: u32,
    height: u32,
    frequency: f64,
    flags: u32,
}

impl Append for GnomeMode {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.id);
            i.append(self.winsys_id);
            i.append(self.width);
            i.append(self.height);
            i.append(self.frequency);
            i.append(self.flags);
        });
    }
}

impl<'a> Get<'a> for GnomeMode {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (id, winsys_id, width, height, frequency, flags) =
            <(u32, i64, u32, u32, f64, u32)>::get(i)?;
        Some(Self {
            id,
            winsys_id,
            width,
            height,
            frequency,
            flags,
        })
    }
}

impl Arg for GnomeMode {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(uxuudu)\0") }
    }
}
