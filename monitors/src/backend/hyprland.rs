// NOTE: This implementation is for the hyprland compositor

use crate::utils::{AvailableMode, Monitor, Size};
use std::process::Command;

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
        .spawn()
        .expect("Could not enable specified monitor");
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
    for mode in available_modes {
        let (resolution, refresh_rate) = mode.split_once('@').unwrap();
        let (resolution_x, resolution_y) = resolution.split_once('x').unwrap();
        let float_hz: f64 = refresh_rate.strip_suffix("Hz").unwrap().parse().unwrap();
        let refresh_rate: u32 = float_hz.round() as u32;
        converted_modes.push(AvailableMode {
            size: Size(resolution_x.parse().unwrap(), resolution_y.parse().unwrap()),
            refresh_rate,
        });
    }
    converted_modes
}
