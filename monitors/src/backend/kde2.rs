use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::sync::Arc;

use wayland_client::backend::ObjectData;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_mode_v2::Event as OutputModeEvent;
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_mode_v2::KdeOutputDeviceModeV2;
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_v2::Event;
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_v2::KdeOutputDeviceV2;
use wayland_protocols_plasma::output_management::v2::client::kde_output_configuration_v2::Event as OutputConfigurationEvent;
use wayland_protocols_plasma::output_management::v2::client::kde_output_configuration_v2::KdeOutputConfigurationV2;
use wayland_protocols_plasma::output_management::v2::client::kde_output_management_v2::Event as OutputManagementEvent;
use wayland_protocols_plasma::output_management::v2::client::kde_output_management_v2::KdeOutputManagementV2;

use crate::utils::{AvailableMode, Monitor, MonitorFeatures, Offset, Size};

const FEATURES: MonitorFeatures = MonitorFeatures {
    vrr: true,
    // Hyprland has no primary monitor concept
    primary: false,
    fractional_scaling: true,
    hdr: false,
};

#[derive(Debug)]
struct AppData {
    heads: HashMap<u32, WlrMonitor>,
    current_monitor: u32,
    current_mode_key: (i32, i32),
    current_mode_refresh_rate: u32,
}

#[derive(Debug)]
struct WlrMonitor {
    name: String,
    make: String,
    model: String,
    serial_number: String,
    description: String,
    offset_x: i32,
    offset_y: i32,
    width: i32,
    height: i32,
    refresh_rate: u32,
    scale: f64,
    modes: HashMap<(i32, i32), WlrMode>,
    vrr: bool,
    enabled: bool,
    transform: u32,
    current_mode: u32,
}

#[derive(Debug)]
struct WlrMode {
    id: u32,
    refresh_rate: HashSet<u32>,
}

impl Dispatch<KdeOutputDeviceModeV2, ()> for AppData {
    fn event(
        data: &mut Self,
        _: &KdeOutputDeviceModeV2,
        event: OutputModeEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        if data.heads.is_empty() {
            return;
        }
        match event {
            OutputModeEvent::Size { width, height } => {
                let len = data.heads.get(&data.current_monitor).unwrap().modes.len() as u32;
                let mode = WlrMode {
                    id: len,
                    refresh_rate: HashSet::new(),
                };
                data.current_mode_key = (width, height);
                if !data
                    .heads
                    .get(&data.current_monitor)
                    .unwrap()
                    .modes
                    .contains_key(&data.current_mode_key)
                {
                    data.heads
                        .get_mut(&data.current_monitor)
                        .unwrap()
                        .modes
                        .insert((width, height), mode);
                }
            }
            OutputModeEvent::Refresh { refresh } => {
                let refresh = refresh / 1000;
                let remainder = refresh % 10;
                let refresh_rate = match remainder {
                    0..=4 => refresh - remainder,
                    5 => refresh,
                    6..=9 => refresh + 10 - remainder,
                    _ => unreachable!(),
                };
                let refresh_rate = refresh_rate as u32;
                data.heads
                    .get_mut(&data.current_monitor)
                    .unwrap()
                    .modes
                    .get_mut(&data.current_mode_key)
                    .unwrap()
                    .refresh_rate
                    .insert(refresh_rate);
                if refresh_rate > data.current_mode_refresh_rate {
                    data.current_mode_refresh_rate = refresh_rate;
                }
            }
            OutputModeEvent::Preferred => {
                let len = data.heads.len() as u32 - 1;
                let monitor = data.heads.get_mut(&data.current_monitor).unwrap();
                monitor.current_mode = len;
                monitor.width = data.current_mode_key.0;
                monitor.height = data.current_mode_key.1;
                monitor.refresh_rate = data.current_mode_refresh_rate;
            }
            _ => (),
        }
    }
}
impl Dispatch<KdeOutputDeviceV2, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &KdeOutputDeviceV2,
        event: Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            Event::Geometry {
                x,
                y,
                physical_width,
                physical_height,
                subpixel,
                make,
                model,
                transform,
            } => {
                let monitor = WlrMonitor {
                    name: String::from(""),
                    make,
                    model,
                    serial_number: String::from(""),
                    description: String::from(""),
                    offset_x: x,
                    offset_y: y,
                    scale: 0.0,
                    modes: HashMap::new(),
                    current_mode: 0,
                    vrr: false,
                    transform: transform as u32,
                    enabled: true,
                    width: 0,
                    height: 0,
                    refresh_rate: 0,
                };
                let len = _state.heads.len() as u32;
                _state.current_monitor = len;
                _state.heads.insert(len, monitor);
            }
            Event::Name { name } => {
                _state.heads.get_mut(&_state.current_monitor).unwrap().name = name;
            }
            // Event::Geometry { x, y, physical_width, physical_height, subpixel, make, model, transform } => todo!(),
            // Event::CurrentMode { mode } => todo!(),
            // Event::Mode { mode } => todo!(),
            // Event::Done => todo!(),
            // Event::Edid { raw } => todo!(),
            // Event::Uuid { uuid } => todo!(),
            // Event::EisaId { eisaId } => todo!(),
            // Event::Capabilities { flags } => todo!(),
            // Event::Overscan { overscan } => todo!(),
            // Event::VrrPolicy { vrr_policy } => todo!(),
            // Event::RgbRange { rgb_range } => todo!(),
            Event::Enabled { enabled } => {
                _state
                    .heads
                    .get_mut(&_state.current_monitor)
                    .unwrap()
                    .enabled = enabled != 0;
            }
            Event::Scale { factor } => {
                _state.heads.get_mut(&_state.current_monitor).unwrap().scale = factor;
            }
            Event::VrrPolicy { vrr_policy } => {
                // 0 is disabled, 1 enabled
                let value: u32 = vrr_policy.into();
                // TODO: make this a proper field -> automatic and always
                _state.heads.get_mut(&_state.current_monitor).unwrap().vrr = value >= 1;
            }
            Event::SerialNumber { serialNumber } => {
                _state
                    .heads
                    .get_mut(&_state.current_monitor)
                    .unwrap()
                    .serial_number = serialNumber;
            }
            _ => (),
        }
    }

    fn event_created_child(_: u16, _qhandle: &QueueHandle<Self>) -> Arc<dyn ObjectData> {
        _qhandle.make_data::<KdeOutputDeviceModeV2, _>(())
    }
}

impl Dispatch<KdeOutputConfigurationV2, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &KdeOutputConfigurationV2,
        _: OutputConfigurationEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}
impl Dispatch<KdeOutputManagementV2, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &KdeOutputManagementV2,
        _: OutputManagementEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppData {
    fn event(
        _: &mut AppData,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        _state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if let "kde_output_device_v2" = &interface[..] {
                println!("{}", interface);
                registry.bind::<KdeOutputDeviceV2, _, _>(name, version, qh, ());
            }
        }
    }
}
pub fn kde2_get_monitor_information() -> Vec<Monitor> {
    let mut monitors = Vec::new();
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, ());

    let mut data = AppData {
        heads: HashMap::new(),
        current_monitor: 0,
        current_mode_key: (0, 0),
        current_mode_refresh_rate: 0,
    };
    // event_queue.roundtrip(&mut data).unwrap();
    event_queue.blocking_dispatch(&mut data).unwrap();
    for (index, wlr_monitor) in data.heads.into_iter() {
        let mut modes = Vec::new();
        for ((width, height), mode) in wlr_monitor.modes.into_iter() {
            let mut refresh_rates: Vec<u32> = mode.refresh_rate.into_iter().collect();
            refresh_rates.sort_unstable_by(|a, b| {
                if a > b {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            });
            modes.push(AvailableMode {
                id: mode.id.to_string(),
                size: Size(width, height),
                refresh_rates,
                supported_scales: Vec::new(),
            });
        }
        modes.sort_unstable_by(|a, b| {
            if a.size > b.size {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });
        let monitor = Monitor {
            id: index,
            enabled: wlr_monitor.enabled,
            name: wlr_monitor.name,
            make: wlr_monitor.make,
            model: wlr_monitor.model,
            serial: wlr_monitor.serial_number,
            refresh_rate: wlr_monitor.refresh_rate,
            scale: wlr_monitor.scale,
            transform: wlr_monitor.transform,
            vrr: wlr_monitor.vrr,
            primary: false,
            offset: Offset(wlr_monitor.offset_x, wlr_monitor.offset_y),
            size: Size(wlr_monitor.width, wlr_monitor.height),
            drag_information: Default::default(),
            mode: wlr_monitor.current_mode.to_string(),
            available_modes: modes,
            features: FEATURES,
        };
        monitors.push(monitor);
    }
    dbg!(&monitors);
    monitors
}
