// NOTE: This implementation is for the hyprland compositor

use crate::utils::Monitor;
use std::process::Command;

pub fn hy_get_monitor_information() -> Vec<Monitor> {
    let mut monitors = Vec::new();
    let hypr_monitors: Vec<HyprMonitor> =
        serde_json::from_str(&String::from_utf8(get_json()).expect("Could not parse json"))
            .expect("Could not parse json");
    for monitor in hypr_monitors {
        monitors.push(monitor.convert_to_regular_monitor())
    }
    monitors
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
            self.refreshRate as u32,
            scale_int,
            scale_float,
            self.transform as u32,
            self.activelyTearing,
            self.vrr,
            self.x as i32,
            self.y as i32,
            self.width as i32,
            self.height as i32,
        )
    }
}
