use std::{fmt::Display, sync::Arc, time::Duration};

use crate::{
    backend::utils::get_wl_backend,
    r#const::{BASE, DBUS_PATH, INTERFACE, SUPPORTED_ENVIRONMENTS},
};
use dbus::{
    arg::{self, Append, Arg, ArgType, Get},
    blocking::Connection,
    Error, Signature,
};
use gtk::prelude::WidgetExt;

use once_cell::sync::Lazy;
use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

pub static ENV: Lazy<String> = Lazy::new(|| {
    get_environment()
});
pub const GNOME: &str = "GNOME";

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
    matches!(get_wl_backend().as_str(), "WLR" | "KWIN")
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
    pub connection: Option<Arc<wayland_client::Connection>>,
}

#[repr(C)]
#[derive(Debug, Clone, Default, PartialEq)]
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
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
pub struct MonitorFeatures {
    pub vrr: bool,
    pub primary: bool,
    pub fractional_scaling: bool,
    pub hdr: bool,
}

impl<'a> Get<'a> for MonitorFeatures {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (vrr, primary, fractional_scaling, hdr) = <(bool, bool, bool, bool)>::get(i)?;
        Some(Self {
            vrr,
            primary,
            fractional_scaling,
            hdr,
        })
    }
}

impl Append for MonitorFeatures {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.vrr);
            i.append(self.primary);
            i.append(self.fractional_scaling);
            i.append(self.hdr);
        });
    }
}

impl Arg for MonitorFeatures {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(bbbb)\0") }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Monitor {
    pub id: u32,
    pub enabled: bool,
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub refresh_rate: u32,
    pub scale: f64,
    pub transform: u32,
    pub vrr: bool,
    pub primary: bool,
    pub offset: Offset,
    pub size: Size,
    pub drag_information: DragInformation,
    pub mode: String,
    pub available_modes: Vec<AvailableMode>,
    pub features: MonitorFeatures,
}

impl Monitor {
    // unlucky
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        enabled: bool,
        name: impl Into<String>,
        make: impl Into<String>,
        model: impl Into<String>,
        serial: impl Into<String>,
        refresh_rate: u32,
        scale: f64,
        transform: u32,
        vrr: bool,
        primary: bool,
        offset_x: i32,
        offset_y: i32,
        width: i32,
        height: i32,
        available_modes: Vec<AvailableMode>,
        features: MonitorFeatures,
    ) -> Self {
        Self {
            id,
            enabled,
            name: name.into(),
            make: make.into(),
            model: model.into(),
            serial: serial.into(),
            refresh_rate,
            scale,
            transform,
            vrr,
            primary,
            offset: Offset(offset_x, offset_y),
            size: Size(width, height),
            mode: "".into(),
            drag_information: DragInformation::default(),
            available_modes,
            features,
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
                ERROR!("Received unsupported transform", ErrorLevel::Recoverable);
                (self.size.0, self.size.1)
            }
        }
    }
}

impl Append for Monitor {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.id);
            i.append(self.enabled);
            i.append((
                self.name.clone(),
                self.make.clone(),
                self.model.clone(),
                self.serial.clone(),
            ));
            i.append((self.refresh_rate, self.scale, self.transform));
            i.append(self.vrr);
            i.append(self.primary);
            i.append(self.offset);
            i.append(self.size);
            i.append(self.mode.clone());
            i.append(self.available_modes.clone());
            i.append(self.features);
        });
    }
}

impl<'a> Get<'a> for Monitor {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (
            id,
            enabled,
            (name, make, model, serial),
            (refresh_rate, scale, transform),
            vrr,
            primary,
            offset,
            size,
            mode,
            available_modes,
            features,
        ) = <(
            u32,
            bool,
            (String, String, String, String),
            (u32, f64, u32),
            bool,
            bool,
            Offset,
            Size,
            String,
            Vec<AvailableMode>,
            MonitorFeatures,
        )>::get(i)?;
        Some(Self {
            id,
            enabled,
            name,
            make,
            model,
            serial,
            refresh_rate,
            scale,
            transform,
            vrr,
            primary,
            offset: Offset(offset.0, offset.1),
            size: Size(size.0, size.1),
            mode,
            drag_information: DragInformation::default(),
            available_modes,
            features,
        })
    }
}

impl Arg for Monitor {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(ub(ssss)(udu)bb(ii)(ii)sa(s(ii)auad)(bbbb))\0") }
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
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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

#[repr(C)]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AvailableMode {
    pub id: String,
    pub size: Size,
    pub refresh_rates: Vec<u32>,
    pub supported_scales: Vec<f64>,
}

impl<'a> Get<'a> for AvailableMode {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (id, size, refresh_rates, supported_scales) =
            <(String, Size, Vec<u32>, Vec<f64>)>::get(i)?;
        Some(Self {
            id,
            size,
            refresh_rates,
            supported_scales,
        })
    }
}

impl Append for AvailableMode {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.id.clone());
            i.append(self.size);
            let sig = unsafe { Signature::from_slice_unchecked("u\0") };
            i.append_array(&sig, |i| {
                for refresh_rate in self.refresh_rates.iter() {
                    i.append(refresh_rate);
                }
            });
            let sig = unsafe { Signature::from_slice_unchecked("d\0") };
            i.append_array(&sig, |i| {
                for scale in self.supported_scales.iter() {
                    i.append(scale);
                }
            });
        });
    }
}

impl Arg for AvailableMode {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(s(ii)auad)\0") }
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

pub struct AlertWrapper {
    pub popup: adw::AlertDialog,
}

unsafe impl Send for AlertWrapper {}
unsafe impl Sync for AlertWrapper {}

impl AlertWrapper {
    pub fn action(&self) {
        self.popup.activate_default();
    }
}

pub fn is_gnome() -> bool {
    ENV.contains(GNOME)
}

#[macro_export]
#[cfg(not(test))]
macro_rules! GNOME_CHECK {
    () => {{}};
}

#[macro_export]
#[cfg(test)]
macro_rules! GNOME_CHECK {
    () => {{
        use $crate::utils::is_gnome;
        if !is_gnome() {
            return false;
        }
    }};
}
