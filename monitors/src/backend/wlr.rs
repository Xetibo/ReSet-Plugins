use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::sync::Arc;

use wayland_client::backend::ObjectData;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::Event as OutputConfigurationEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::Event as OutputManagerEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as OutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

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

impl Dispatch<ZwlrOutputModeV1, ()> for AppData {
    fn event(
        data: &mut Self,
        _: &ZwlrOutputModeV1,
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
impl Dispatch<ZwlrOutputHeadV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &ZwlrOutputHeadV1,
        event: Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            Event::Name { name } => {
                let monitor = WlrMonitor {
                    name,
                    make: String::from(""),
                    model: String::from(""),
                    serial_number: String::from(""),
                    description: String::from(""),
                    offset_x: 0,
                    offset_y: 0,
                    scale: 0.0,
                    modes: HashMap::new(),
                    current_mode: 0,
                    vrr: false,
                    transform: 0,
                    enabled: true,
                    width: 0,
                    height: 0,
                    refresh_rate: 0,
                };
                let len = _state.heads.len() as u32;
                _state.current_monitor = len;
                _state.heads.insert(len, monitor);
            }
            Event::Description { description } => {
                _state
                    .heads
                    .get_mut(&_state.current_monitor)
                    .unwrap()
                    .description = description;
            }
            Event::Enabled { enabled } => {
                _state
                    .heads
                    .get_mut(&_state.current_monitor)
                    .unwrap()
                    .enabled = enabled != 0;
            }
            Event::Position { x, y } => {
                let monitor = _state.heads.get_mut(&_state.current_monitor).unwrap();
                monitor.offset_x = x;
                monitor.offset_y = y;
            }
            Event::Transform { transform } => {
                // ReSet transform are already wayland spec conforming, no conversion necessary,
                // just convert enum to int. -> enums can't be sent over DBus directly
                _state
                    .heads
                    .get_mut(&_state.current_monitor)
                    .unwrap()
                    .transform = transform.into();
            }
            Event::Scale { scale } => {
                _state.heads.get_mut(&_state.current_monitor).unwrap().scale = scale;
            }
            Event::Finished => {
                println!("monitor done");
            }
            Event::AdaptiveSync { state } => {
                // 0 is disabled, 1 enabled
                let value: u32 = state.into();
                _state.heads.get_mut(&_state.current_monitor).unwrap().vrr = value == 1;
            }
            Event::Make { make } => {
                _state.heads.get_mut(&_state.current_monitor).unwrap().make = make;
            }
            Event::Model { model } => {
                _state.heads.get_mut(&_state.current_monitor).unwrap().model = model;
            }
            Event::SerialNumber { serial_number } => {
                _state
                    .heads
                    .get_mut(&_state.current_monitor)
                    .unwrap()
                    .serial_number = serial_number;
            }
            _ => (),
        }
    }

    fn event_created_child(_: u16, _qhandle: &QueueHandle<Self>) -> Arc<dyn ObjectData> {
        _qhandle.make_data::<ZwlrOutputModeV1, _>(())
    }
}

impl Dispatch<ZwlrOutputManagerV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &ZwlrOutputManagerV1,
        _: OutputManagerEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }

    fn event_created_child(_: u16, _qhandle: &QueueHandle<Self>) -> Arc<dyn ObjectData> {
        _qhandle.make_data::<ZwlrOutputHeadV1, _>(())
    }
}
impl Dispatch<ZwlrOutputConfigurationV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &ZwlrOutputConfigurationV1,
        _: OutputConfigurationEvent,
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
            if let "zwlr_output_manager_v1" = &interface[..] {
                registry.bind::<ZwlrOutputManagerV1, _, _>(name, version, qh, ());
            }
        }
    }
}
pub fn wlr_get_monitor_information() -> Vec<Monitor> {
    let mut monitors = Vec::new();
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut queue) = registry_queue_init::<AppData>(&conn).unwrap();
    let handle = queue.handle();
    globals
        .bind::<ZwlrOutputManagerV1, _, _>(&handle, RangeInclusive::new(0, 1), ())
        .unwrap();

    let mut data = AppData {
        heads: HashMap::new(),
        current_monitor: 0,
        current_mode_key: (0, 0),
        current_mode_refresh_rate: 0,
    };
    queue.blocking_dispatch(&mut data).unwrap();
    for (index, wlr_monitor) in data.heads.into_iter() {
        let mut modes = Vec::new();
        for ((width, height), mode) in wlr_monitor.modes.into_iter() {
            modes.push(AvailableMode {
                id: mode.id.to_string(),
                size: Size(width, height),
                refresh_rates: mode.refresh_rate.into_iter().collect(),
                supported_scales: Vec::new(),
            });
        }
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
