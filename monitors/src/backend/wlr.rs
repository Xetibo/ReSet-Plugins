use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::sync::Arc;

use wayland_client::backend::{ObjectData, ObjectId};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_output::Transform;
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::Event as OutputConfigurationHeadEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::Event as OutputConfigurationEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::Event as OutputManagerEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as OutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::utils::{AvailableMode, Monitor, MonitorFeatures, Offset, Size};

const FEATURES: MonitorFeatures = MonitorFeatures {
    // NOTE: this function currently causes a crash on the wayland library
    vrr: false,
    // wlr has no primary monitor concept
    primary: false,
    fractional_scaling: true,
    hdr: false,
};

struct TransformWrapper(Transform);

impl TransformWrapper {
    fn value(self) -> Transform {
        self.0
    }
}

impl From<u32> for TransformWrapper {
    fn from(val: u32) -> Self {
        TransformWrapper(match val {
            0 => Transform::Normal,
            1 => Transform::_90,
            2 => Transform::_180,
            3 => Transform::_270,
            4 => Transform::Flipped,
            5 => Transform::Flipped90,
            6 => Transform::Flipped180,
            7 => Transform::Flipped270,
            _ => unreachable!(),
        })
    }
}

struct CurrentMode {
    pub id: Cell<u32>,
    pub refresh_rate: Cell<u32>,
    pub width: Cell<i32>,
    pub height: Cell<i32>,
}

unsafe impl Send for CurrentMode {}
unsafe impl Sync for CurrentMode {}

#[derive(Debug)]
struct AppData {
    heads: HashMap<u32, WlrMonitor>,
    current_monitor: u32,
    current_mode_key: (i32, i32),
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
    original_object: ObjectId,
    current_mode_object: Option<ObjectId>,
    hash_modes: HashMap<u32, ObjectId>,
    next_mode: u32,
}

#[derive(Debug)]
struct WlrMode {
    id: u32,
    refresh_rate: HashSet<(u32, String)>,
}

impl Dispatch<ZwlrOutputModeV1, CurrentMode> for AppData {
    fn event(
        data: &mut Self,
        obj: &ZwlrOutputModeV1,
        event: OutputModeEvent,
        current: &CurrentMode,
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        if data.heads.is_empty() {
            return;
        }
        match event {
            OutputModeEvent::Size { width, height } => {
                let mode = WlrMode {
                    id: data.heads.get(&data.current_monitor).unwrap().next_mode,
                    refresh_rate: HashSet::new(),
                };
                data.current_mode_key = (width, height);
                current.width.replace(width);
                current.height.replace(height);
                current.id.replace(mode.id);
                if !data
                    .heads
                    .get(&data.current_monitor)
                    .unwrap()
                    .modes
                    .contains_key(&data.current_mode_key)
                {
                    let monitor = data.heads.get_mut(&data.current_monitor).unwrap();
                    monitor.modes.insert((width, height), mode);
                    monitor.hash_modes.insert(monitor.next_mode, obj.id());
                }
            }
            OutputModeEvent::Refresh { refresh } => {
                let refresh = refresh / 1000;
                let remainder = refresh % 10;
                // set the refresh rate to be a multiple of 5
                let refresh_rate = match remainder {
                    0..=4 => refresh - remainder,
                    5 => refresh,
                    6..=9 => refresh + 10 - remainder,
                    _ => unreachable!(),
                };
                let refresh_rate = refresh_rate as u32;
                current.refresh_rate.replace(refresh_rate);
                let len;
                let new;
                {
                    let monitor = data.heads.get(&data.current_monitor).unwrap();
                    len = monitor.next_mode;
                    let refresh_rates = monitor.modes.get(&data.current_mode_key).unwrap();
                    // check if the current or the previous entry already has this refresh rate and
                    // id
                    new = !refresh_rates
                        .refresh_rate
                        .contains(&(refresh_rate, (len).to_string()))
                        && !refresh_rates.refresh_rate.contains(&(
                            refresh_rate,
                            (if len == 0 { 0 } else { len - 1 }).to_string(),
                        ));
                }
                if new {
                    // insert refresh rate and the id
                    let monitor = data.heads.get_mut(&data.current_monitor).unwrap();
                    monitor.hash_modes.insert(len, obj.id());
                    monitor
                        .modes
                        .get_mut(&data.current_mode_key)
                        .unwrap()
                        .refresh_rate
                        .insert((refresh_rate, len.to_string()));
                    data.heads.get_mut(&data.current_monitor).unwrap().next_mode = len + 1;
                }
            }
            _ => (),
        }
    }
}
impl Dispatch<ZwlrOutputHeadV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        obj: &ZwlrOutputHeadV1,
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
                    original_object: obj.id(),
                    current_mode_object: None,
                    hash_modes: HashMap::new(),
                    next_mode: 0,
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
            Event::CurrentMode { mode } => {
                let data: &CurrentMode = mode.data().unwrap();
                let monitor = _state.heads.get_mut(&_state.current_monitor).unwrap();
                monitor.width = data.width.take();
                monitor.height = data.height.take();
                monitor.refresh_rate = data.refresh_rate.take();
                monitor.current_mode = data.id.take();
                monitor.current_mode_object = Some(mode.id());
            }
            _ => (),
        }
    }

    fn event_created_child(_: u16, _qhandle: &QueueHandle<Self>) -> Arc<dyn ObjectData> {
        _qhandle.make_data::<ZwlrOutputModeV1, CurrentMode>(CurrentMode {
            id: Cell::new(0),
            refresh_rate: Cell::new(0),
            width: Cell::new(0),
            height: Cell::new(0),
        })
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
        event: OutputConfigurationEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            OutputConfigurationEvent::Succeeded => (),
            OutputConfigurationEvent::Failed => {
                ERROR!("Could not apply configuration", ErrorLevel::Recoverable);
            }
            OutputConfigurationEvent::Cancelled => (),
            _ => unreachable!(),
        }
    }
}
impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &ZwlrOutputConfigurationHeadV1,
        _: OutputConfigurationHeadEvent,
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

pub fn wlr_get_monitor_information(conn: Option<Arc<wayland_client::Connection>>) -> Vec<Monitor> {
    if conn.is_none() {
        return Vec::new();
    }
    let mut monitors = Vec::new();
    let (globals, mut queue) = registry_queue_init::<AppData>(&conn.clone().unwrap()).unwrap();
    let handle = queue.handle();
    let manager = globals.bind::<ZwlrOutputManagerV1, _, _>(&handle, RangeInclusive::new(0, 1), ());
    if manager.is_err() {
        return Vec::new();
    }
    let _ = manager.unwrap();

    let mut data = AppData {
        heads: HashMap::new(),
        current_monitor: 0,
        current_mode_key: (0, 0),
    };
    queue.blocking_dispatch(&mut data).unwrap();
    for (index, wlr_monitor) in data.heads.into_iter() {
        let mut modes = Vec::new();
        for ((width, height), mode) in wlr_monitor.modes.into_iter() {
            let mut refresh_rates: Vec<(u32, String)> = mode.refresh_rate.into_iter().collect();
            refresh_rates.sort_unstable_by(|a, b| {
                if a < b {
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
            if a.size < b.size {
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
            uses_mode_id: true,
            features: FEATURES,
        };
        monitors.push(monitor);
    }
    queue.flush().unwrap();
    monitors
}

pub fn wlr_apply_monitor_configuration(
    conn: Option<Arc<wayland_client::Connection>>,
    monitors: &[Monitor],
) {
    if conn.is_none() {
        return;
    }
    let conn = conn.clone().unwrap();
    let (globals, mut queue) = registry_queue_init::<AppData>(&conn).unwrap();
    let handle = queue.handle();
    let manager = globals.bind::<ZwlrOutputManagerV1, _, _>(&handle, RangeInclusive::new(0, 1), ());
    if manager.is_err() {
        return;
    }
    let configuration = manager.unwrap().create_configuration(0, &handle, ());

    let mut data = AppData {
        heads: HashMap::new(),
        current_monitor: 0,
        current_mode_key: (0, 0),
    };
    queue.blocking_dispatch(&mut data).unwrap();
    for monitor in monitors.iter() {
        for (id, head) in data.heads.iter() {
            if monitor.id == *id {
                let current_head =
                    ZwlrOutputHeadV1::from_id(&conn, head.original_object.clone()).unwrap();
                // enable or disable monitors
                if !monitor.enabled {
                    configuration.disable_head(&current_head);
                    continue;
                }
                let head_configuration = configuration.enable_head(&current_head, &handle, ());

                // get the mode id back, and apply the mode
                // mode is size and refresh rate
                let current_mode = monitor.mode.parse::<u32>().unwrap();
                let mode_id = head.hash_modes.get(&current_mode).unwrap();
                head_configuration
                    .set_mode(&ZwlrOutputModeV1::from_id(&conn, mode_id.clone()).unwrap());

                let transform: TransformWrapper = monitor.transform.into();
                head_configuration.set_transform(transform.value());
                head_configuration.set_scale(monitor.scale);
                head_configuration.set_position(monitor.offset.0, monitor.offset.1);

                // This causes an error on hyprland as of now
                // enabling or disabling vrr for monitors that do not offer vrr doesn't work
                // if monitor.features.vrr {
                // if monitor.vrr {
                //     head_configuration.set_adaptive_sync(AdaptiveSyncState::Enabled);
                // } else {
                //     head_configuration.set_adaptive_sync(AdaptiveSyncState::Disabled);
                // }
                // }
            }
        }
    }
    configuration.apply();
    queue.blocking_dispatch(&mut data).unwrap();
    queue.flush().unwrap();
}
