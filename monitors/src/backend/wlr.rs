use std::cell::Cell;
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
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::{
    AdaptiveSyncState, Event,
};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::Event as OutputManagerEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as OutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::utils::{AvailableMode, Monitor, MonitorFeatures, Offset, Size};

const FEATURES: MonitorFeatures = MonitorFeatures {
    vrr: true,
    // Hyprland has no primary monitor concept
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

impl Into<TransformWrapper> for u32 {
    fn into(self) -> TransformWrapper {
        TransformWrapper(match self {
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
    original_object: ObjectId,
    current_mode_object: Option<ObjectId>,
    hash_modes: HashMap<u32, ObjectId>,
    next_mode: u32,
}

#[derive(Debug)]
struct WlrMode {
    id: u32,
    refresh_rate: HashSet<u32>,
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
                let refresh_rate = match remainder {
                    0..=4 => refresh - remainder,
                    5 => refresh,
                    6..=9 => refresh + 10 - remainder,
                    _ => unreachable!(),
                };
                let refresh_rate = refresh_rate as u32 - 1;
                current.refresh_rate.replace(refresh_rate);
                let len = data.heads.get(&data.current_monitor).unwrap().next_mode;
                let new = data
                    .heads
                    .get_mut(&data.current_monitor)
                    .unwrap()
                    .modes
                    .get_mut(&data.current_mode_key)
                    .unwrap()
                    .refresh_rate
                    .insert(refresh_rate);
                if new && data.heads.get(&data.current_monitor).unwrap().modes.len() != 1 {
                    data.heads
                        .get_mut(&data.current_monitor)
                        .unwrap()
                        .hash_modes
                        .insert(len, obj.id());
                    data.heads.get_mut(&data.current_monitor).unwrap().next_mode = len + 1;
                }
                if refresh_rate > data.current_mode_refresh_rate {
                    data.current_mode_refresh_rate = refresh_rate;
                }
            }
            OutputModeEvent::Preferred => {
                // let monitor = data.heads.get_mut(&data.current_monitor).unwrap();
                // let len = monitor.modes.len() as u32 - 1;
                // monitor.current_mode = len;
                // monitor.width = data.current_mode_key.0;
                // monitor.height = data.current_mode_key.1;
                // monitor.refresh_rate = data.current_mode_refresh_rate;
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
            Event::CurrentMode { mode } => {
                let data: &CurrentMode = mode.data().unwrap();
                let monitor = _state.heads.get_mut(&_state.current_monitor).unwrap();
                monitor.width = data.width.take();
                monitor.height = data.height.take();
                monitor.refresh_rate = data.refresh_rate.take();
                monitor.current_mode_object = Some(mode.id());
            }
            _ => (),
        }
    }

    fn event_created_child(_: u16, _qhandle: &QueueHandle<Self>) -> Arc<dyn ObjectData> {
        _qhandle.make_data::<ZwlrOutputModeV1, CurrentMode>(CurrentMode {
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
        dbg!(&wlr_monitor.hash_modes);
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
            wl_object_ids: wlr_monitor.hash_modes.clone(),
        };
        monitors.push(monitor);
    }
    monitors
}

pub fn wlr_apply_monitor_configuration(
    monitors: &[Monitor],
    wlr_objects_vec: &[HashMap<u32, ObjectId>],
) {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut queue) = registry_queue_init::<AppData>(&conn).unwrap();
    let handle = queue.handle();
    let manager = globals
        .bind::<ZwlrOutputManagerV1, _, _>(&handle, RangeInclusive::new(0, 1), ())
        .unwrap();
    let configuration = manager.create_configuration(0, &handle, ());

    let mut data = AppData {
        heads: HashMap::new(),
        current_monitor: 0,
        current_mode_key: (0, 0),
        current_mode_refresh_rate: 0,
    };
    queue.blocking_dispatch(&mut data).unwrap();
    for (monitor, wlr_objects) in monitors.iter().zip(wlr_objects_vec) {
        for head in data.heads.iter() {
            if monitor.id == *head.0 {
                let current_head =
                    ZwlrOutputHeadV1::from_id(&conn, head.1.original_object.clone()).unwrap();
                if !monitor.enabled {
                    configuration.disable_head(&current_head);
                    continue;
                }
                let head_configuration = configuration.enable_head(&current_head, &handle, ());
                let transform: TransformWrapper = monitor.transform.into();

                dbg!(wlr_objects);
                let current_mode = monitor.mode.parse::<u32>().unwrap();
                let mode_id = wlr_objects.get(&current_mode).unwrap();
                head_configuration
                    .set_mode(&ZwlrOutputModeV1::from_id(&conn, mode_id.clone()).unwrap());
                // head_configuration.set_custom_mode(
                //     monitor.size.0,
                //     monitor.size.1,
                //     monitor.refresh_rate as i32 * 1000,
                // );
                head_configuration.set_transform(transform.value());
                head_configuration.set_scale(monitor.scale);
                head_configuration.set_position(monitor.offset.0, monitor.offset.1);

                // enabling or disabling vrr for monitors that do not offer vrr doesn't work
                if monitor.features.vrr {
                    if monitor.vrr {
                        head_configuration.set_adaptive_sync(AdaptiveSyncState::Enabled);
                    } else {
                        head_configuration.set_adaptive_sync(AdaptiveSyncState::Disabled);
                    }
                }
            }
        }
        configuration.apply();
    }
    queue.blocking_dispatch(&mut data).unwrap();
}
