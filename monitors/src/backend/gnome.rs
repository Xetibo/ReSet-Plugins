use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    time::Duration,
};

use dbus::{
    arg::{self, prop_cast, Append, Arg, ArgType, Get, PropMap},
    blocking::Connection,
    Error, Signature,
};
use gtk::prelude::SettingsExtManual;
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file, ERROR};

use crate::utils::{AvailableMode, DragInformation, Monitor, MonitorFeatures, Offset, Size};

const BASE: &str = "org.gnome.Mutter.DisplayConfig";
const DBUS_PATH: &str = "/org/gnome/Mutter/DisplayConfig";
const INTERFACE: &str = "org.gnome.Mutter.DisplayConfig";

fn get_fractional_scale_support() -> bool {
    let settings = gtk::gio::Settings::new("org.gnome.mutter");
    let features = settings.strv("experimental-features");
    for value in features {
        if value == "scale-monitor-framebuffer" {
            return true;
        }
    }
    false
}

fn get_variable_refresh_rate_support() -> bool {
    let settings = gtk::gio::Settings::new("org.gnome.mutter");
    let features = settings.strv("experimental-features");
    for value in features {
        if value == "variable-refresh-rate" {
            return true;
        }
    }
    false
}

pub fn g_get_monitor_information() -> Vec<Monitor> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(u32, Vec<GnomeMonitor>, Vec<GnomeLogicalMonitor>, PropMap), Error> =
        proxy.method_call(INTERFACE, "GetCurrentState", ());
    if res.is_err() {
        ERROR!("Could fetch monitor configuration", ErrorLevel::Recoverable);
    }
    let (serial, monitors, logical_monitors, _properties) = res.unwrap();
    let gnome_monitors = GnomeMonitorConfig {
        serial,
        monitors,
        logical_monitors,
        _properties,
    };
    gnome_monitors.inplace_to_regular_monitor()
}

pub fn g_apply_monitor_config(apply_mode: u32, monitors: &Vec<Monitor>) {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(), Error> = proxy.method_call(
        INTERFACE,
        "ApplyMonitorsConfig",
        GnomeMonitorConfig::from_regular_monitor(apply_mode, monitors),
    );
    if res.is_err() {
        ERROR!(
            "Could not apply monitor configuration",
            ErrorLevel::Recoverable
        );
    }
}

#[derive(Debug)]
pub struct GnomeMonitorConfig {
    serial: u32,
    monitors: Vec<GnomeMonitor>,
    logical_monitors: Vec<GnomeLogicalMonitor>,
    _properties: PropMap,
}

impl GnomeMonitorConfig {
    fn inplace_to_regular_monitor(self) -> Vec<Monitor> {
        let mut monitors = Vec::new();
        for (monitor, logical_monitor) in self
            .monitors
            .into_iter()
            .zip(self.logical_monitors.into_iter())
        {
            let empty_mode = GnomeMode {
                id: "-1".into(),
                width: 0,
                height: 0,
                refresh_rate: 0.0,
                _scale: 0.0,
                supported_scales: Vec::new(),
                properties: PropMap::new(),
            };
            let mut hash_modes: HashMap<Size, (String, HashSet<u32>, Vec<f64>)> = HashMap::new();
            let mut modes = Vec::new();
            let mut current_mode: Option<&GnomeMode> = None;
            let mut enabled = false;
            for mode in monitor.modes.iter() {
                let flag_opt: Option<&bool> = prop_cast(&mode.properties, "is-current");
                if let Some(flag) = flag_opt {
                    if *flag {
                        current_mode = Some(mode);
                        enabled = true;
                    }
                }
                if let Some(saved_mode) = hash_modes.get_mut(&Size(mode.width, mode.height)) {
                    saved_mode.1.insert(mode.refresh_rate.round() as u32);
                } else {
                    let mut refresh_rates = HashSet::new();
                    refresh_rates.insert(mode.refresh_rate.round() as u32);
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
            if current_mode.is_none() {
                current_mode = Some(&empty_mode);
            }
            let current_mode = current_mode.unwrap();
            let vrr_enabled = get_variable_refresh_rate_support();
            let mut vrr = false;
            let refresh_rate_opt: Option<&String> =
                prop_cast(&current_mode.properties, "refresh-rate-mode");
            if let Some(refresh_rate_mode) = refresh_rate_opt {
                println!("{}", refresh_rate_mode);
                if refresh_rate_mode == "variable" && vrr_enabled {
                    vrr = true;
                }
            }

            monitors.push(Monitor {
                id: self.serial,
                enabled,
                name: monitor.name.connector,
                make: monitor.name.vendor,
                model: monitor.name.product,
                serial: monitor.name.serial,
                refresh_rate: current_mode.refresh_rate.round() as u32,
                scale: logical_monitor.scale,
                transform: logical_monitor.transform,
                // TODO: bug
                vrr,
                primary: logical_monitor.primary,
                offset: Offset(logical_monitor.x, logical_monitor.y),
                size: Size(current_mode.width, current_mode.height),
                mode: current_mode.id.clone(),
                drag_information: DragInformation::default(),
                available_modes: modes,
                features: MonitorFeatures {
                    vrr: vrr_enabled,
                    // Gnome requires a primary monitor to be set
                    primary: true,
                    fractional_scaling: get_fractional_scale_support(),
                },
            });
        }
        monitors
    }

    pub fn from_regular_monitor(
        apply_mode: u32,
        monitors: &Vec<Monitor>,
    ) -> (u32, u32, Vec<GnomeLogicalMonitorSend>, PropMap) {
        let mut g_logical_monitors = Vec::new();
        let id = monitors.first().unwrap().id;
        for monitor in monitors {
            let mode = if monitor.enabled {
                monitor.mode.clone()
            } else {
                "-1".into()
            };
            g_logical_monitors.push(GnomeLogicalMonitorSend {
                x: monitor.offset.0,
                y: monitor.offset.1,
                scale: monitor.scale,
                transform: monitor.transform,
                primary: monitor.primary,
                // TODO: propmap
                monitors: vec![(monitor.name.clone(), mode, PropMap::new())],
            });
        }
        (id, apply_mode, g_logical_monitors, PropMap::new())
    }
}

#[derive(Debug)]
pub struct GnomeMonitor {
    name: GnomeName,
    modes: Vec<GnomeMode>,
    _properties: PropMap,
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
#[derive(Debug)]
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

#[derive(Debug)]
pub struct GnomeLogicalMonitor {
    x: i32,
    y: i32,
    scale: f64,
    transform: u32,
    primary: bool,
    _monitors: Vec<(String, String, String, String)>,
    _properties: PropMap,
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
