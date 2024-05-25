use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use wayland_client::backend::{ObjectData, ObjectId};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_callback::{self};
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_mode_v2::Event as OutputModeEvent;
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_mode_v2::KdeOutputDeviceModeV2;
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_v2::Event;
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_v2::KdeOutputDeviceV2;
use wayland_protocols_plasma::output_management::v2::client::kde_output_configuration_v2::KdeOutputConfigurationV2;
use wayland_protocols_plasma::output_management::v2::client::kde_output_configuration_v2::{
    Event as OutputConfigurationEvent, VrrPolicy,
};
use wayland_protocols_plasma::output_management::v2::client::kde_output_management_v2::Event as OutputManagementEvent;
use wayland_protocols_plasma::output_management::v2::client::kde_output_management_v2::KdeOutputManagementV2;

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
// This is a conversion struct, hence the fields need to be there either way
#[allow(dead_code)]
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

impl Dispatch<KdeOutputDeviceModeV2, CurrentMode> for AppData {
    fn event(
        data: &mut Self,
        obj: &KdeOutputDeviceModeV2,
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
                let refresh_rate = refresh_rate as u32;
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
                make,
                model,
                transform,
                ..
            } => {
                let monitor = _state.heads.get_mut(&_state.current_monitor).unwrap();
                monitor.make = make;
                monitor.model = model;
                monitor.offset_x = x;
                monitor.offset_y = y;
                monitor.transform = transform as u32;
            }
            Event::Name { name } => {
                _state.heads.get_mut(&_state.current_monitor).unwrap().name = name;
            }
            Event::CurrentMode { mode } => {
                // data passed to each mode
                let data: &CurrentMode = mode.data().unwrap();
                // if the mode is the current mode, apply needed info to monitor
                let monitor = _state.heads.get_mut(&_state.current_monitor).unwrap();
                monitor.width = data.width.take();
                monitor.height = data.height.take();
                monitor.refresh_rate = data.refresh_rate.take();
                monitor.current_mode_object = Some(mode.id());
            }
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
        _qhandle.make_data::<KdeOutputDeviceModeV2, CurrentMode>(CurrentMode {
            // create data for each mode
            refresh_rate: Cell::new(0),
            width: Cell::new(0),
            height: Cell::new(0),
        })
    }
}

impl Dispatch<KdeOutputConfigurationV2, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &KdeOutputConfigurationV2,
        event: OutputConfigurationEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            OutputConfigurationEvent::Applied => (),
            OutputConfigurationEvent::Failed => {
                ERROR!("Could not apply configuration", ErrorLevel::Recoverable);
            }
            _ => unreachable!(),
        }
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

impl Dispatch<wl_callback::WlCallback, ()> for AppData {
    fn event(
        _: &mut AppData,
        _: &wl_callback::WlCallback,
        _: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}

pub fn kwin_get_monitor_information() -> Vec<Monitor> {
    let mut monitors = Vec::new();
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut queue) = registry_queue_init::<AppData>(&conn).unwrap();
    let handle = queue.handle();

    let mut data = AppData {
        heads: HashMap::new(),
        current_monitor: 0,
        current_mode_key: (0, 0),
        current_mode_refresh_rate: 0,
    };

    for global in globals.contents().clone_list() {
        if &global.interface[..] == "kde_output_device_v2" {
            let output: KdeOutputDeviceV2 =
                globals
                    .registry()
                    .bind::<KdeOutputDeviceV2, _, _>(global.name, 2, &handle, ());
            let monitor = WlrMonitor {
                name: String::from(""),
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
                original_object: output.id(),
                current_mode_object: None,
                hash_modes: HashMap::new(),
                next_mode: 0,
            };
            let len = data.heads.len() as u32;
            data.current_monitor = len;
            data.heads.insert(len, monitor);

            queue.blocking_dispatch(&mut data).unwrap();
        }
    }

    for (index, kwin_monitor) in data.heads.into_iter() {
        let mut modes = Vec::new();
        for ((width, height), mode) in kwin_monitor.modes.into_iter() {
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
            enabled: kwin_monitor.enabled,
            name: kwin_monitor.name,
            make: kwin_monitor.make,
            model: kwin_monitor.model,
            serial: kwin_monitor.serial_number,
            refresh_rate: kwin_monitor.refresh_rate,
            scale: kwin_monitor.scale,
            transform: kwin_monitor.transform,
            vrr: kwin_monitor.vrr,
            primary: false,
            offset: Offset(kwin_monitor.offset_x, kwin_monitor.offset_y),
            size: Size(kwin_monitor.width, kwin_monitor.height),
            drag_information: Default::default(),
            mode: kwin_monitor.current_mode.to_string(),
            available_modes: modes,
            features: FEATURES,
            wl_object_ids: kwin_monitor.hash_modes.clone(),
        };
        monitors.push(monitor);
    }
    monitors
}

pub fn kwin_apply_monitor_configuration(
    monitors: &[Monitor],
    kwin_objects_vec: &[HashMap<u32, ObjectId>],
) {
    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut queue) = registry_queue_init::<AppData>(&conn).unwrap();
    let handle = queue.handle();

    let manager = globals.bind::<KdeOutputManagementV2, _, _>(&handle, 1..=2, ());
    if manager.is_err() {
        return;
    }
    let configuration = manager.unwrap().create_configuration(&handle, ());

    let mut data = AppData {
        heads: HashMap::new(),
        current_monitor: 0,
        current_mode_key: (0, 0),
        current_mode_refresh_rate: 0,
    };

    for (monitor, kwin_objects) in monitors.iter().zip(kwin_objects_vec) {
        for head in data.heads.iter() {
            if monitor.id == *head.0 {
                let current_head =
                    KdeOutputDeviceV2::from_id(&conn, head.1.original_object.clone()).unwrap();
                if !monitor.enabled {
                    configuration.enable(&current_head, 0);
                    continue;
                }
                configuration.enable(&current_head, 1);

                let current_mode = monitor.mode.parse::<u32>().unwrap();
                let mode_id = kwin_objects.get(&current_mode).unwrap();
                configuration.mode(
                    &current_head,
                    &KdeOutputDeviceModeV2::from_id(&conn, mode_id.clone()).unwrap(),
                );

                configuration.transform(&current_head, monitor.transform as i32);
                configuration.position(&current_head, monitor.offset.0, monitor.offset.1);
                configuration.scale(&current_head, monitor.scale);
                let vrr = if monitor.vrr {
                    VrrPolicy::Automatic
                } else {
                    VrrPolicy::Never
                };
                configuration.set_vrr_policy(&current_head, vrr);
                if monitor.primary {
                    configuration.set_primary_output(&current_head);
                }
            }
        }
    }
    configuration.apply();
    queue.blocking_dispatch(&mut data).unwrap();
}
