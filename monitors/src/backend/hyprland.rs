// NOTE: This implementation is for the hyprland compositor

use re_set_lib::utils::config::CONFIG;

use crate::utils::{get_environment, AvailableMode, Monitor, Size};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::Command,
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

pub fn apply_monitor_configuration(monitors: &Vec<Monitor>) {
    match get_environment().as_str() {
        "Hyprland" => hy_apply_monitor_information(monitors),
        _ => println!("Environment not supported!"),
    };
}

pub fn hy_apply_monitor_information(monitors: &Vec<Monitor>) {
    Command::new("hyprctl")
        .args(["--batch", &monitor_to_configstring(monitors)])
        .spawn()
        .expect("Could not enable specified monitor");
}

pub fn save_monitor_configuration(monitors: &Vec<Monitor>) {
    match get_environment().as_str() {
        "Hyprland" => hy_save_monitor_configuration(monitors),
        _ => println!("Environment not supported!"),
    };
}

pub fn get_default_path() -> String {
    let dirs = directories_next::ProjectDirs::from("org", "Xetibo", "ReSet").unwrap();
    let buf = dirs.config_dir().join("monitor.conf");
    let path = buf.to_str().unwrap();
    String::from(path)
}
pub fn hy_save_monitor_configuration(monitors: &Vec<Monitor>) {
    let path;
    if let Some(test) = CONFIG.get("Monitor").unwrap().get("path") {
        path = test.as_str().unwrap().to_string();
    } else {
        path = get_default_path();
    }

    let mut input_config = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(PathBuf::from(path))
        .expect("Failed to open file");

    let mut layout_string = String::new();
    let mut variant_string = String::new();

    let string = format!("pingpang");

    input_config.set_len(0).expect("Failed to truncate file");
    input_config
        .write_all(string.as_bytes())
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
    availableModes: Vec<String>,
}

impl HyprMonitor {
    fn convert_to_regular_monitor(self) -> Monitor {
        let scale_int = self.scale as u32;
        let scale_float = (self.scale - scale_int as f64) as u32 * 1000;
        Monitor::new(
            self.id as u32,
            self.name,
            self.make,
            self.model,
            self.serial,
            self.refreshRate.round() as u32,
            scale_int,
            scale_float,
            self.transform as u32,
            self.activelyTearing,
            self.vrr,
            self.x as i32,
            self.y as i32,
            self.width as i32,
            self.height as i32,
            string_to_modes(self.availableModes),
        )
    }
}

fn monitor_to_configstring(monitors: &Vec<Monitor>) -> String {
    let mut strings = Vec::new();

    for monitor in monitors {
        strings.push(format!(
            "keyword monitor {},{}x{}@{},{}x{},{}.{},transform,{};",
            monitor.name,
            &monitor.size.0.to_string(),
            &monitor.size.1.to_string(),
            &monitor.refresh_rate.to_string(),
            &monitor.offset.0.to_string(),
            &monitor.offset.1.to_string(),
            &monitor.scale.0.to_string(),
            &monitor.scale.1.to_string(),
            &monitor.transform.to_string()
        ));
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
            size: Size(resolution_x.parse().unwrap(), resolution_y.parse().unwrap()),
            refresh_rates,
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
