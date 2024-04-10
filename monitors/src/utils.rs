use std::{fmt::Display, time::Duration};

use dbus::{
    arg::{self, Append, Arg, ArgType, Get},
    blocking::Connection,
    Error, Signature,
};

use crate::r#const::{BASE, DBUS_PATH, INTERFACE, SUPPORTED_ENVIRONMENTS};

pub fn get_environment() -> String {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP");
    if desktop.is_err() {
        return "NONE".into();
    }
    desktop.unwrap()
}

pub fn check_environment_support() -> bool {
    let desktop = get_environment();
    if SUPPORTED_ENVIRONMENTS.contains(&desktop.as_str()) {
        return true;
    }
    false
}

pub fn get_monitor_data() -> Vec<Monitor> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(Vec<Monitor>,), Error> = proxy.method_call(INTERFACE, "GetMonitors", ());
    dbg!(&res);
    if res.is_err() {
        return Vec::new();
    }
    res.unwrap().0
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MonitorData {
    pub monitors: Vec<Monitor>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Monitor {
    pub id: u32,
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub refresh_rate: u32,
    pub scale: Scale,
    pub transform: u32,
    pub vrr: bool,
    pub tearing: bool,
    pub offset: Offset,
    pub size: Size,
}

impl Monitor {
    // unlucky
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        name: impl Into<String>,
        make: impl Into<String>,
        model: impl Into<String>,
        serial: impl Into<String>,
        refresh_rate: u32,
        scale_int: u32,
        scale_float: u32,
        transform: u32,
        vrr: bool,
        tearing: bool,
        offset_x: i32,
        offset_y: i32,
        width: i32,
        height: i32,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            make: make.into(),
            model: model.into(),
            serial: serial.into(),
            refresh_rate,
            scale: Scale(scale_int, scale_float),
            transform,
            vrr,
            tearing,
            offset: Offset(offset_x, offset_y),
            size: Size(width, height),
        }
    }

    pub fn handle_transform(&self) -> (i32, i32) {
        match self.transform {
            0 => (self.size.0, self.size.1),
            1 => (self.size.1, self.size.0),
            2 => (self.size.0, self.size.1),
            3 => (self.size.1, self.size.0),
            4 => (self.size.0, self.size.1),
            5 => (self.size.1, self.size.0),
            6 => (self.size.0, self.size.1),
            7 => (self.size.1, self.size.0),
            _ => {
                println!("got an unsupported transform");
                (self.size.0, self.size.1)
            }
        }
    }
}

impl Append for Monitor {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.id);
            i.append(self.name.clone());
            i.append(self.make.clone());
            i.append(self.model.clone());
            i.append(self.serial.clone());
            i.append(self.refresh_rate);
            i.append(self.scale);
            i.append(self.transform);
            i.append(self.tearing);
            i.append(self.vrr);
            i.append(self.offset);
            i.append(self.size);
        });
    }
}

impl<'a> Get<'a> for Monitor {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (
            id,
            name,
            make,
            model,
            serial,
            refresh_rate,
            scale,
            transform,
            vrr,
            tearing,
            offset,
            size,
        ) = <(
            u32,
            String,
            String,
            String,
            String,
            u32,
            Scale,
            u32,
            bool,
            bool,
            Offset,
            Size,
        )>::get(i)?;
        Some(Self {
            id,
            name,
            make,
            model,
            serial,
            refresh_rate,
            scale: Scale(scale.0, scale.1),
            transform,
            vrr,
            tearing,
            offset: Offset(offset.0, offset.1),
            size: Size(size.0, size.1),
        })
    }
}

impl Arg for Monitor {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ussssu(uu)ubb(ii)(ii))\0") }
    }
}

impl Monitor {
    /// These coordinates are calculated from the edge of the drawing box. Ensure the rest of the
    /// window is also taken into account when passing parameters as it will otherwise evaluate to
    /// false.
    pub fn is_coordinate_within(&self, x: i32, y: i32) -> bool {
        x >= self.offset.0
            && x <= self.offset.0 + self.size.0
            && y >= self.offset.1
            && y <= self.offset.1 + self.size.1
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Offset(pub i32, pub i32);

impl<'a> Get<'a> for Offset {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (x, y) = <(i32, i32)>::get(i)?;
        Some(Self(x, y))
    }
}

impl Append for Offset {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.0);
            i.append(self.1);
        });
    }
}

impl Arg for Offset {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ii)\0") }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Size(pub i32, pub i32);

impl<'a> Get<'a> for Size {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (width, height) = <(i32, i32)>::get(i)?;
        Some(Self(width, height))
    }
}

impl Append for Size {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.0);
            i.append(self.1);
        });
    }
}

impl Arg for Size {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ii)\0") }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Scale(pub u32, pub u32);

impl<'a> Get<'a> for Scale {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (x, y) = <(u32, u32)>::get(i)?;
        Some(Self(x, y))
    }
}

impl Append for Scale {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.0);
            i.append(self.1);
        });
    }
}

impl Arg for Scale {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(uu)\0") }
    }
}

// TODO: move this to reset-lib
#[derive(Debug)]
pub struct PluginInstantiationError(&'static str);

impl PluginInstantiationError {
    pub fn message(&self) -> &'static str {
        self.0
    }

    pub fn new(message: &'static str) -> Self {
        Self(message)
    }
}

impl Default for PluginInstantiationError {
    fn default() -> Self {
        Self("Environment not supported")
    }
}

impl Display for PluginInstantiationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
