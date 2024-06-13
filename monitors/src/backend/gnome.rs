use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    process::Command,
    time::Duration,
};

use dbus::{
    arg::{self, prop_cast, Append, Arg, ArgType, Get, PropMap},
    blocking::Connection,
    Error, Signature,
};

use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::utils::{AvailableMode, DragInformation, Monitor, MonitorFeatures, Offset, Size};

const BASE: &str = "org.gnome.Mutter.DisplayConfig";
const DBUS_PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const INTERFACE: &str = "org.gnome.Mutter.DisplayConfig";

pub fn gnome_features() -> MonitorFeatures {
    let experimental_features = get_experimental_support();
    MonitorFeatures {
        vrr: experimental_features.1,
        // Gnome requires a primary monitor to be set
        primary: true,
        fractional_scaling: experimental_features.0,
        hdr: false,
    }
}

fn get_experimental_support() -> (bool, bool) {
    let command = Command::new("gsettings")
        .args(["get", "org.gnome.mutter", "experimental-features"])
        .output();
    match command {
        Ok(_) => {
            let command = command.unwrap();
            let command = String::from_utf8(command.stdout).unwrap();
            (
                command.contains("scale-monitor-framebuffer"),
                command.contains("variable-refresh-rate"),
            )
        }
        Err(_) => {
            let command = Command::new("flatpak-spawn")
                .args([
                    "--host",
                    "gsettings",
                    "get",
                    "org.gnome.mutter",
                    "experimental-features",
                ])
                .output();

            if command.is_err() {
                return (false, false);
            }
            let command = command.unwrap();
            let command = String::from_utf8(command.stdout).unwrap();
            (
                command.contains("scale-monitor-framebuffer"),
                command.contains("variable-refresh-rate"),
            )
        }
    }
}

pub fn g_get_monitor_information(serial: &mut u32) -> Vec<Monitor> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(5000));
    let res: Result<(u32, Vec<GnomeMonitor>, Vec<GnomeLogicalMonitor>, PropMap), Error> =
        proxy.method_call(INTERFACE, "GetCurrentState", ());
    if res.is_err() {
        ERROR!("Could fetch monitor configuration", ErrorLevel::Recoverable);
        return Vec::new();
    }
    let (fetched_serial, monitors, logical_monitors, _properties) = res.unwrap();
    *serial = fetched_serial;
    let gnome_monitors = GnomeMonitorConfig {
        serial: fetched_serial,
        monitors,
        logical_monitors,
        _properties,
    };
    gnome_monitors.inplace_to_regular_monitor()
}

pub fn g_apply_monitor_config(apply_mode: u32, monitors: &Vec<Monitor>) {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(5000));
    let res: Result<(u32, Vec<GnomeMonitor>, Vec<GnomeLogicalMonitor>, PropMap), Error> =
        proxy.method_call(INTERFACE, "GetCurrentState", ());
    if res.is_err() {
        ERROR!("Could fetch monitor configuration", ErrorLevel::Recoverable);
        return;
    }
    let serial = res.unwrap().0;
    let res: Result<(), Error> = proxy.method_call(
        INTERFACE,
        "ApplyMonitorsConfig",
        GnomeMonitorConfig::from_regular_monitor(serial, apply_mode, monitors),
    );
    if let Err(_error) = res {
        ERROR!(
            format!("Could not apply monitor configuration {}", _error),
            ErrorLevel::Recoverable
        );
    }
}

#[derive(Debug, Default)]
pub struct GnomeMonitorConfig {
    pub serial: u32,
    pub monitors: Vec<GnomeMonitor>,
    pub logical_monitors: Vec<GnomeLogicalMonitor>,
    pub _properties: PropMap,
}

impl GnomeMonitorConfig {
    pub fn inplace_to_regular_monitor(self) -> Vec<Monitor> {
        type HashModes = HashMap<Size, (String, HashSet<(u32, String)>, Vec<f64>)>;
        let mut monitors = Vec::new();
        let mut monitor_iter = self.monitors.into_iter();
        let mut logical_iter = self.logical_monitors.into_iter().peekable();
        let mut count = 0;
        let features = gnome_features();
        loop {
            let monitor = monitor_iter.next();
            if monitor.is_none() {
                break;
            }
            let monitor = monitor.unwrap();
            let first_mode = monitor.modes.first();
            if first_mode.is_none() {
                continue;
            }
            let empty_mode = GnomeMode {
                id: first_mode.unwrap().id.clone(),
                width: 500,
                height: 500,
                refresh_rate: 0.0,
                _scale: 0.0,
                supported_scales: Vec::new(),
                properties: PropMap::new(),
            };
            let mut hash_modes: HashModes = HashMap::new();
            let mut modes = Vec::new();
            let mut current_mode: Option<&GnomeMode> = None;
            for mode in monitor.modes.iter() {
                let flag_opt: Option<&bool> = prop_cast(&mode.properties, "is-current");
                if let Some(flag) = flag_opt {
                    if *flag {
                        current_mode = Some(mode);
                    }
                }
                if let Some(saved_mode) = hash_modes.get_mut(&Size(mode.width, mode.height)) {
                    saved_mode
                        .1
                        .insert((mode.refresh_rate.round() as u32, mode.id.clone()));
                } else {
                    let mut refresh_rates = HashSet::new();
                    refresh_rates.insert((mode.refresh_rate.round() as u32, mode.id.clone()));
                    hash_modes.insert(
                        Size(mode.width, mode.height),
                        (
                            mode.id.clone(),
                            refresh_rates,
                            mode.supported_scales.clone(),
                        ),
                    );
                }
            }
            for (size, (id, refresh_rates, supported_scales)) in hash_modes {
                let mut refresh_rates: Vec<(u32, String)> = refresh_rates.into_iter().collect();
                refresh_rates.sort_unstable_by(|a, b| {
                    if a.0 < b.0 {
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
                if a.size < b.size {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            });
            if current_mode.is_none() {
                current_mode = Some(&empty_mode);
            }
            let current_mode = current_mode.unwrap();
            let mut vrr = false;
            let refresh_rate_opt: Option<&String> =
                prop_cast(&current_mode.properties, "refresh-rate-mode");
            if let Some(refresh_rate_mode) = refresh_rate_opt {
                if refresh_rate_mode == "variable" && features.vrr {
                    vrr = true;
                }
            }

            let mut hasher = DefaultHasher::new();
            monitor.name.connector.hash(&mut hasher);
            let id = hasher.finish();

            let mut enabled = false;
            let maybe_logical_monitor = logical_iter.peek();
            if let Some(logical_monitor) = maybe_logical_monitor {
                for names in logical_monitor._monitors.iter() {
                    if names.0 == monitor.name.connector {
                        enabled = true;
                    }
                }
            }
            if enabled {
                let logical_monitor = logical_iter.next().unwrap();
                monitors.push(Monitor {
                    id: id as u32,
                    enabled,
                    name: monitor.name.connector,
                    make: monitor.name.vendor,
                    model: monitor.name.product,
                    serial: monitor.name.serial,
                    refresh_rate: current_mode.refresh_rate.round() as u32,
                    scale: logical_monitor.scale,
                    transform: logical_monitor.transform,
                    vrr,
                    primary: logical_monitor.primary,
                    offset: Offset(logical_monitor.x, logical_monitor.y),
                    size: Size(current_mode.width, current_mode.height),
                    mode: current_mode.id.clone(),
                    drag_information: DragInformation::default(),
                    available_modes: modes,
                    uses_mode_id: true,
                    features,
                });
            } else {
                count += 1;
                monitors.push(Monitor {
                    id: id as u32,
                    enabled,
                    name: monitor.name.connector,
                    make: monitor.name.vendor,
                    model: monitor.name.product,
                    serial: monitor.name.serial,
                    refresh_rate: current_mode.refresh_rate.round() as u32,
                    scale: 1.0,
                    transform: 0,
                    vrr,
                    primary: false,
                    offset: Offset(count * -500 + -50, 0),
                    size: Size(current_mode.width, current_mode.height),
                    mode: current_mode.id.clone(),
                    drag_information: DragInformation::default(),
                    available_modes: modes,
                    uses_mode_id: true,
                    features,
                });
            }
        }
        monitors
    }

    pub fn from_regular_monitor(
        serial: u32,
        apply_mode: u32,
        monitors: &Vec<Monitor>,
    ) -> (u32, u32, Vec<GnomeLogicalMonitorSend>, PropMap) {
        let mut g_logical_monitors = Vec::new();
        for monitor in monitors {
            if !monitor.enabled {
                continue;
            }
            g_logical_monitors.push(GnomeLogicalMonitorSend {
                x: monitor.offset.0,
                y: monitor.offset.1,
                scale: monitor.scale,
                transform: monitor.transform,
                primary: monitor.primary,
                monitors: vec![(monitor.name.clone(), monitor.mode.clone(), PropMap::new())],
            });
        }
        (serial, apply_mode, g_logical_monitors, PropMap::new())
    }
}

#[derive(Debug, Default)]
pub struct GnomeMonitor {
    pub name: GnomeName,
    pub modes: Vec<GnomeMode>,
    pub _properties: PropMap,
}

impl<'a> Get<'a> for GnomeMonitor {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (name, modes, properties) = <(GnomeName, Vec<GnomeMode>, PropMap)>::get(i)?;
        Some(Self {
            name,
            modes,
            _properties: properties,
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
#[derive(Debug, Default)]
pub struct GnomeMode {
    id: String,
    width: i32,
    height: i32,
    refresh_rate: f64,
    _scale: f64,
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
            _scale: scale,
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

#[derive(Debug, Default)]
pub struct GnomeLogicalMonitor {
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: u32,
    pub primary: bool,
    pub _monitors: Vec<(String, String, String, String)>,
    pub _properties: PropMap,
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
            _monitors: monitors,
            _properties: properties,
        })
    }
}

impl Arg for GnomeLogicalMonitor {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(iiduba(ssss)a{sv})\0") }
    }
}

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
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
