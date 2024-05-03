use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    process::Command,
    rc::Rc,
};

use crate::utils::{AvailableMode, Monitor, MonitorFeatures, Offset, Size};

pub fn kde_get_monitor_information() -> Vec<Monitor> {
    let mut monitors = Vec::new();
    let kde_monitors: KDEMonitorConfiguration =
        serde_json::from_str(&String::from_utf8(get_json()).expect("Could not parse json"))
            .expect("Could not parse json");
    for monitor in kde_monitors.outputs {
        let monitor = monitor.convert_to_regular_monitor();
        monitors.push(monitor);
    }
    monitors
}

fn get_json() -> Vec<u8> {
    Command::new("kscreen-doctor")
        .args(["-j"])
        .output()
        .expect("Could not retrieve monitor json")
        .stdout
}

pub fn kde_apply_monitor_config(monitors: &Vec<Monitor>) {}

pub fn kde_save_monitor_config(monitors: &Vec<Monitor>) {}

pub fn kde_add_scaling_adjustment(
    scale: f64,
    monitor_index: usize,
    scaling_ref: Rc<RefCell<Vec<Monitor>>>,
    settings: &adw::PreferencesGroup,
) {
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct KDEMonitorConfiguration {
    outputs: Vec<KDEMonitor>,
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct KDEMonitor {
    id: u32,
    name: String,
    enabled: bool,
    connected: bool,
    scale: f64,
    rotation: u32,
    x: i32,
    y: i32,
    priority: u32,
    currentModeId: String,
    available_modes: Vec<KDEMode>,
}

impl KDEMonitor {
    pub fn convert_to_regular_monitor(self) -> Monitor {
        let mode = self
            .available_modes
            .get(self.currentModeId.parse::<u32>().unwrap() as usize)
            .unwrap();
        Monitor {
            id: self.id,
            name: self.name,
            make: "None".into(),
            model: "None".into(),
            serial: "None".into(),
            refresh_rate: mode.refreshRate.round() as u32,
            scale: self.scale,
            transform: self.rotation,
            // TODO: how to get this?
            vrr: false,
            primary: self.priority == 1,
            offset: Offset(self.x, self.y),
            size: Size(mode.width, mode.height),
            drag_information: Default::default(),
            mode: self.currentModeId,
            available_modes: convert_modes(self.available_modes),
            features: MonitorFeatures {
                // KDE supports all the features!
                vrr: true,
                primary: true,
                fractional_scaling: true,
            },
        }
    }
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct KDEMode {
    id: String,
    refreshRate: f64,
    height: i32,
    width: i32,
}

fn convert_modes(kde_modes: Vec<KDEMode>) -> Vec<AvailableMode> {
    let mut modes = Vec::new();
    let mut hash_modes: HashMap<(i32, i32), (HashSet<u32>, String)> = HashMap::new();

    for mode in kde_modes {
        if let Some(hash_mode) = hash_modes.get_mut(&(mode.width, mode.height)) {
            hash_mode.0.insert(mode.refreshRate.round() as u32);
        } else {
            let mut refresh_rates = HashSet::new();
            refresh_rates.insert(mode.refreshRate.round() as u32);
            hash_modes.insert((mode.width, mode.height), (refresh_rates, mode.id));
        }
    }

    for ((width, height), (refresh_rates, id)) in hash_modes {
        modes.push(AvailableMode {
            id,
            size: Size(width, height),
            refresh_rates: refresh_rates.into_iter().collect(),
            supported_scales: Vec::new(),
        });
    }

    modes
}
