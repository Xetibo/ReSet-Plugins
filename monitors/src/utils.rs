use std::{fmt::Display, time::Duration};

use crate::r#const::{BASE, DBUS_PATH, INTERFACE, SUPPORTED_ENVIRONMENTS};
use dbus::{
    arg::{self, Append, Arg, ArgType, Get},
    blocking::Connection,
    Error, Signature,
};

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
#[derive(Debug, Clone, Default)]
pub struct DragInformation {
    pub drag_x: i32,
    pub drag_y: i32,
    pub border_offset_x: i32,
    pub border_offset_y: i32,
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: i32,
    pub height: i32,
    pub factor: i32,
    pub drag_active: bool,
    pub clicked: bool,
    pub changed: bool,
    pub prev_scale: f64,
}

#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct Monitor {
    pub id: u32,
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub refresh_rate: u32,
    pub scale: f64,
    pub transform: u32,
    pub vrr: bool,
    pub tearing: bool,
    pub offset: Offset,
    pub size: Size,
    pub drag_information: DragInformation,
    pub mode: String,
    pub available_modes: Vec<AvailableMode>,
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
        scale: f64,
        transform: u32,
        vrr: bool,
        tearing: bool,
        offset_x: i32,
        offset_y: i32,
        width: i32,
        height: i32,
        available_modes: Vec<AvailableMode>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            make: make.into(),
            model: model.into(),
            serial: serial.into(),
            refresh_rate,
            scale,
            transform,
            vrr,
            tearing,
            offset: Offset(offset_x, offset_y),
            size: Size(width, height),
            mode: "".into(),
            drag_information: DragInformation::default(),
            available_modes,
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
            i.append((
                self.name.clone(),
                self.make.clone(),
                self.model.clone(),
                self.serial.clone(),
            ));
            i.append(self.refresh_rate);
            i.append(self.scale);
            i.append(self.transform);
            i.append(self.tearing);
            i.append(self.vrr);
            i.append(self.offset);
            i.append(self.size);
            i.append(self.mode.clone());
            i.append(self.available_modes.clone())
        });
    }
}

impl<'a> Get<'a> for Monitor {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (
            id,
            (name, make, model, serial),
            refresh_rate,
            scale,
            transform,
            vrr,
            tearing,
            offset,
            size,
        mode,
            available_modes,
        ) = <(
            u32,
            (String, String, String, String),
            u32,
            f64,
            u32,
            bool,
            bool,
            Offset,
            Size,
            String,
            Vec<AvailableMode>,
        )>::get(i)?;
        Some(Self {
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
            offset: Offset(offset.0, offset.1),
            size: Size(size.0, size.1),
            mode,
            drag_information: DragInformation::default(),
            available_modes,
        })
    }
}

impl Arg for Monitor {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(u(ssss)udubb(ii)(ii)sa(s(ii)au))\0") }
    }
}

impl Monitor {
    /// These coordinates are calculated from the edge of the drawing box. Ensure the rest of the
    /// window is also taken into account when passing parameters as it will otherwise evaluate to
    /// false.
    pub fn is_coordinate_within(&self, x: i32, y: i32) -> bool {
        let offset_x = self.offset.0 / self.drag_information.factor;
        let offset_y = self.offset.1 / self.drag_information.factor;
        let height = self.drag_information.height / self.drag_information.factor;
        let width = self.drag_information.width / self.drag_information.factor;
        x >= self.drag_information.border_offset_x + offset_x
            && x <= self.drag_information.border_offset_x + offset_x + width
            && y >= self.drag_information.border_offset_y + offset_y
            && y <= self.drag_information.border_offset_y + offset_y + height
    }

    /// Checks whether or not the currently dragged monitor has any overlap with existing monitors.
    /// If this is the case, then the monitor should be reset to original position on drop.
    pub fn intersect_horizontal(&self, offset_x: i32, width: i32) -> bool {
        // current monitor left side is right of other right
        let left = self.drag_information.border_offset_x + self.offset.0 >= offset_x + width;
        // current monitor right is left of other left
        let right =
            self.drag_information.border_offset_x + self.offset.0 + self.drag_information.width
                <= offset_x;
        !left && !right
    }

    pub fn intersect_vertical(&self, offset_y: i32, height: i32) -> bool {
        // current monitor bottom is higher than other top
        let bottom = self.drag_information.border_offset_y + self.offset.1 >= offset_y + height;
        // current monitor top is lower than other bottom
        let top =
            self.drag_information.border_offset_y + self.offset.1 + self.drag_information.height
                <= offset_y;
        !bottom && !top
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, Default)]
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

impl Display for Scale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}.{}", self.0, self.1))
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

#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct AvailableMode {
    pub id: String,
    pub size: Size,
    pub refresh_rates: Vec<u32>,
}

impl<'a> Get<'a> for AvailableMode {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (id, size, refresh_rates) = <(String, Size, Vec<u32>)>::get(i)?;
        Some(Self {
            id,
            size,
            refresh_rates,
        })
    }
}

impl Append for AvailableMode {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.size);
            let sig = unsafe { Signature::from_slice_unchecked("u\0") };
            i.append_array(&sig, |i| {
                for refresh_rate in self.refresh_rates.iter() {
                    i.append(refresh_rate);
                }
            });
        });
    }
}

impl Arg for AvailableMode {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(s(ii)au)\0") }
    }
}

#[derive(Eq, PartialEq, PartialOrd, Ord)]
pub enum SnapDirectionHorizontal {
    RightRight(i32),
    RightLeft(i32),
    LeftLeft(i32),
    LeftRight(i32),
    None,
}

#[derive(Eq, PartialEq, PartialOrd, Ord)]
pub enum SnapDirectionVertical {
    TopTop(i32),
    TopBottom(i32),
    BottomBottom(i32),
    BottomTop(i32),
    None,
}
