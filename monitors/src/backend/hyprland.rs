// NOTE: This implementation is for the hyprland compositor

use re_set_lib::utils::config::CONFIG;

use crate::utils::{AvailableMode, Monitor, MonitorFeatures, Size};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

const FEATURES: MonitorFeatures = MonitorFeatures {
    vrr: true,
    // Hyprland has no primary monitor concept
    primary: false,
    fractional_scaling: true,
    full_transform: true,
};

pub fn hy_get_monitor_information() -> Vec<Monitor> {
    let mut monitors = Vec::new();
    let hypr_monitors: Vec<HyprMonitor> =
        serde_json::from_str(&String::from_utf8(get_json()).expect("Could not parse json"))
            .expect("Could not parse json");
    for monitor in hypr_monitors {
        let monitor = monitor.convert_to_regular_monitor();
        monitors.push(monitor);
    }
    monitors
}

pub fn hy_apply_monitor_information(monitors: &Vec<Monitor>) {
    Command::new("hyprctl")
        .args(["--batch", &monitor_to_configstring(monitors)])
        .stdout(Stdio::null())
        .spawn()
        .expect("Could not enable specified monitor");
}

fn get_default_path() -> String {
    let dirs = directories_next::ProjectDirs::from("org", "Xetibo", "ReSet").unwrap();
    let buf = dirs.config_dir().join("monitor.conf");
    let path = buf.to_str().unwrap();
    String::from(path)
}

pub fn hy_save_monitor_configuration(monitors: &Vec<Monitor>) {
    let config = CONFIG;
    let path;
    if let Some(config) = config.get("Monitor") {
        if let Some(test) = config.get("path") {
            path = test.as_str().unwrap().to_string();
        } else {
            path = get_default_path();
        }
    } else {
        path = get_default_path();
    }

    let mut input_config = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(true)
        .open(PathBuf::from(path))
        .expect("Failed to open file");

    let mut monitor_string = String::new();

    for monitor in monitors {
        if !monitor.enabled {
            monitor_string += &format!("keyword monitor {},disabled;", monitor.name);
        } else {
            monitor_string += &format!(
                "monitor={},{}x{}@{},{}x{},{:.6},transform,{}\n",
                monitor.name,
                monitor.size.0,
                monitor.size.1,
                monitor.refresh_rate,
                monitor.offset.0,
                monitor.offset.1,
                monitor.scale,
                monitor.transform,
            );
        }
    }

    input_config
        .write_all(monitor_string.as_bytes())
        .expect("Failed to write to file");
    input_config.sync_all().expect("Failed to sync file");
}

fn get_json() -> Vec<u8> {
    Command::new("hyprctl")
        .args(["-j", "monitors"])
        .output()
        .expect("Could not retrieve monitor json")
        .stdout
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct HyprMonitor {
    id: i64,
    name: String,
    description: String,
    make: String,
    model: String,
    serial: String,
    width: i64,
    height: i64,
    refreshRate: f64,
    x: i64,
    y: i64,
    scale: f64,
    transform: i64,
    vrr: bool,
    activelyTearing: bool,
    disabled: bool,
    availableModes: Vec<String>,
}

impl HyprMonitor {
    fn convert_to_regular_monitor(self) -> Monitor {
        Monitor::new(
            self.id as u32,
            !self.disabled,
            self.name,
            self.make,
            self.model,
            self.serial,
            self.refreshRate.round() as u32,
            self.scale,
            self.transform as u32,
            self.activelyTearing,
            self.vrr,
            self.x as i32,
            self.y as i32,
            self.width as i32,
            self.height as i32,
            string_to_modes(self.availableModes),
            FEATURES,
        )
    }
}

fn monitor_to_configstring(monitors: &Vec<Monitor>) -> String {
    let mut strings = Vec::new();

    for monitor in monitors {
        if !monitor.enabled {
            strings.push(format!("keyword monitor {},disabled;", monitor.name));
        } else {
            strings.push(format!(
                "keyword monitor {},{}x{}@{},{}x{},{:.6},transform,{};",
                monitor.name,
                &monitor.size.0,
                &monitor.size.1,
                &monitor.refresh_rate,
                &monitor.offset.0,
                &monitor.offset.1,
                &monitor.scale,
                &monitor.transform
            ));
        }
    }

    strings.concat()
}

fn string_to_modes(available_modes: Vec<String>) -> Vec<AvailableMode> {
    let mut converted_modes = Vec::new();
    let mut resolutions: HashMap<&str, HashSet<u32>> = HashMap::new();
    for mode in available_modes.iter() {
        let (resolution, refresh_rate) = mode.split_once('@').unwrap();
        let entry = resolutions.get_mut(resolution);
        if let Some(entry) = entry {
            let float_hz: f64 = refresh_rate.strip_suffix("Hz").unwrap().parse().unwrap();
            let refresh_rates: u32 = float_hz.round() as u32;
            entry.insert(refresh_rates);
            continue;
        }
        resolutions.insert(resolution, HashSet::new());
        let entry = resolutions.get_mut(resolution).unwrap();
        let float_hz: f64 = refresh_rate.strip_suffix("Hz").unwrap().parse().unwrap();
        let refresh_rates: u32 = float_hz.round() as u32;
        entry.insert(refresh_rates);
    }
    for (resolution, refresh_rates) in resolutions {
        let (resolution_x, resolution_y) = resolution.split_once('x').unwrap();
        let mut refresh_rates: Vec<u32> = refresh_rates.into_iter().collect();
        refresh_rates.sort_unstable();
        refresh_rates.reverse();
        converted_modes.push(AvailableMode {
            id: "".into(),
            size: Size(resolution_x.parse().unwrap(), resolution_y.parse().unwrap()),
            refresh_rates,
            // Hyprland allows arbitrary scales and hence no supported scales are provided
            supported_scales: Vec::new(),
        });
    }
    converted_modes.sort_unstable_by(|a, b| {
        if a.size > b.size {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });
    converted_modes
}
