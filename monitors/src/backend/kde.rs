use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    process::Command,
};

use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

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
    let command = Command::new("kscreen-doctor").args(["-j"]).output();
    if let Ok(command) = command {
        return command.stdout;
    }
    ERROR!(
        "Kscreen is not installed, please install kscreen for kde.",
        ErrorLevel::PartialBreakage
    );
    Vec::new()
}

pub fn kde_apply_monitor_config(monitors: &Vec<Monitor>) {
    kde_save_monitor_config(monitors);
}

pub fn kde_save_monitor_config(monitors: &Vec<Monitor>) {
    let args = convert_modes_to_kscreen_string(monitors);
    Command::new("kscreen-doctor")
        .args(args)
        .spawn()
        .expect("Could not retrieve monitor json");
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct KDEMonitorConfiguration {
    outputs: Vec<KDEMonitor>,
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct KDEMonitor {
    id: u32,
    name: String,
    enabled: bool,
    connected: bool,
    scale: f64,
    rotation: u32,
    pos: KDEOffset,
    priority: u32,
    vrrPolicy: Option<u32>,
    currentModeId: String,
    modes: Vec<KDEMode>,
}

impl KDEMonitor {
    pub fn convert_to_regular_monitor(self) -> Monitor {
        let modes = convert_modes(&self.currentModeId, self.modes);
        let vrr = if let Some(_vrr) = self.vrrPolicy {
            // TODO: apparently KDE offers 2 vrr versions -> automatic and always
            // todo is to handle both states instead of a bool right now
            true
        } else {
            // NOTE: KDE does not even show the VRR option within the json fetching if no the
            // monitor can't handle VRR either way
            false
        };
        Monitor {
            id: self.id,
            enabled: self.enabled,
            name: self.name,
            // TODO: check if KDE has some other method to retrieve this
            // from the regular kscreen-doctor, there is no fetching for this
            make: "".into(),
            model: "".into(),
            serial: "".into(),
            refresh_rate: modes.1.refreshRate.round() as u32,
            scale: self.scale,
            transform: convert_to_regular_transform(self.rotation),
            // TODO: how to get this?
            vrr,
            primary: self.priority == 1,
            offset: self.pos.convert_to_regular_offset(),
            size: modes.1.size.convert_to_regular_size(),
            drag_information: Default::default(),
            mode: self.currentModeId,
            available_modes: modes.0,
            features: MonitorFeatures {
                // KDE supports all the features!
                vrr: true,
                primary: true,
                fractional_scaling: true,
                hdr: true,
            },
            wl_object_ids: HashMap::new(),
        }
    }
}

fn convert_to_regular_transform(rotation: u32) -> u32 {
    match rotation {
        1 => 0,
        2 => 1,
        3 => 2,
        4 => 3,
        _ => {
            ERROR!(
                "Passed invalid transform value for KDE.",
                ErrorLevel::Recoverable
            );
            0
        }
    }
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct KDEMode {
    id: String,
    refreshRate: f64,
    size: KDESize,
}

impl KDESize {
    pub fn convert_to_regular_size(self) -> Size {
        Size(self.width, self.height)
    }
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct KDEOffset {
    x: i32,
    y: i32,
}

impl KDEOffset {
    pub fn convert_to_regular_offset(self) -> Offset {
        Offset(self.x, self.y)
    }
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct KDESize {
    width: i32,
    height: i32,
}

fn convert_modes(
    current_mode_id: &String,
    kde_modes: Vec<KDEMode>,
) -> (Vec<AvailableMode>, KDEMode) {
    let mut modes = Vec::new();
    let mut current_mode: Option<KDEMode> = None;
    let mut hash_modes: HashMap<(i32, i32), (HashSet<u32>, String)> = HashMap::new();

    for mode in kde_modes {
        if &mode.id == current_mode_id {
            current_mode = Some(mode.clone());
        }
        if let Some(hash_mode) = hash_modes.get_mut(&(mode.size.width, mode.size.height)) {
            hash_mode.0.insert(mode.refreshRate.round() as u32);
        } else {
            let mut refresh_rates = HashSet::new();
            refresh_rates.insert(mode.refreshRate.round() as u32);
            hash_modes.insert(
                (mode.size.width, mode.size.height),
                (refresh_rates, mode.id),
            );
        }
    }

    for ((width, height), (refresh_rates, id)) in hash_modes {
        let mut refresh_rates: Vec<u32> = refresh_rates.into_iter().collect();
        refresh_rates.sort_unstable_by(|a, b| {
            if a > b {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });
        let mode = AvailableMode {
            id,
            size: Size(width, height),
            refresh_rates,
            supported_scales: Vec::new(),
        };
        modes.push(mode);
    }

    modes.sort_unstable_by(|a, b| {
        if a.size > b.size {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });

    if current_mode.is_none() {
        return (modes, KDEMode::default());
    }
    (modes, current_mode.unwrap())
}

fn convert_modes_to_kscreen_string(monitors: &Vec<Monitor>) -> Vec<String> {
    let mut kscreen = Vec::new();

    for monitor in monitors {
        let rotation = match monitor.transform {
            0 => "none",
            1 => "right",
            2 => "inverted",
            3 => "left",
            4 => "flipped",
            5 => "flipped90",
            6 => "flipped180",
            7 => "flipped270",
            _ => unreachable!(),
        };
        let start = format!("output.{}.", monitor.name);
        if !monitor.enabled {
            kscreen.push(start.clone() + &format!("enable.{}", monitor.enabled));
        } else {
            kscreen.push(
                start.clone()
                    + &format!(
                        "mode.{}x{}@{}",
                        monitor.size.0, monitor.size.1, monitor.refresh_rate
                    ),
            );
            kscreen.push(start.clone() + &format!("scale.{}", monitor.scale));
            kscreen.push(
                start.clone() + &format!("position.{},{}", monitor.offset.0, monitor.offset.1),
            );
            kscreen.push(start + &format!("rotation.{}", rotation));
        }
    }

    kscreen
}
