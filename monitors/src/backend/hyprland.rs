// NOTE: This implementation is for the hyprland compositor

use re_set_lib::{utils::config::CONFIG, ERROR};
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::utils::{is_flatpak, AvailableMode, Monitor, MonitorFeatures, Size};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use super::wlr::{wlr_apply_monitor_configuration, wlr_get_monitor_information};

pub const HYPRFEATURES: MonitorFeatures = MonitorFeatures {
    vrr: true,
    // Hyprland has no primary monitor concept
    primary: false,
    fractional_scaling: true,
    hdr: false,
};

// Due to hyprland moving away from WLR, ReSet chose to fetch data via hyprctl instead.
// The tool is also always installed for hyprland.
pub fn hy_get_monitor_information(
    conn: Option<std::sync::Arc<wayland_client::Connection>>,
) -> Vec<Monitor> {
    let mut monitors = Vec::new();
    let json = get_json();

    if json.is_err() {
        return wlr_get_monitor_information(conn);
    }

    let json = json.expect("Could not retrieve monitor json").stdout;
    let json_string = String::from_utf8(json);
    if let Ok(json_string) = json_string {
        let hypr_monitors: Result<Vec<HyprMonitor>, _> = serde_json::from_str(&json_string);
        if hypr_monitors.is_err() {
            ERROR!(
                "Failed to deserialize to monitor datastructure",
                ErrorLevel::PartialBreakage
            );
            return Vec::new();
        }
        for monitor in hypr_monitors.unwrap() {
            let monitor = monitor.convert_to_regular_monitor();
            monitors.push(monitor);
        }
    } else {
        ERROR!(
            "Failed to get string from json",
            ErrorLevel::PartialBreakage
        );
    }

    monitors
}

// The same applies to applying
pub fn hy_apply_monitor_information(
    monitors: &Vec<Monitor>,
    conn: Option<std::sync::Arc<wayland_client::Connection>>,
) {
    let config_string = monitor_to_configstring(monitors);

    let command = if is_flatpak() {
        Command::new("flatpak-spawn")
            .args(["--host", "hyprctl", "--batch", &config_string])
            .stdout(Stdio::null())
            .spawn()
    } else {
        Command::new("hyprctl")
            .args(["--batch", &config_string])
            .stdout(Stdio::null())
            .spawn()
    };
    match command.is_err() {
        true => {
            wlr_apply_monitor_configuration(conn, monitors);
        }
        false => {
            command.unwrap();
        }
    }
}

fn get_default_path() -> String {
    let dirs = directories_next::ProjectDirs::from("org", "Xetibo", "ReSet").unwrap();
    let buf = dirs.config_dir().join("monitor.conf");
    let path = buf.to_str().unwrap();
    String::from(path)
}

// saving can only be done via configuration file and hence is not supported via the wlr protocol
// either way
pub fn hy_save_monitor_configuration(monitors: &Vec<Monitor>) {
    let path;
    if let Some(config) = CONFIG.get("Monitor") {
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
        let vrr = if monitor.vrr { 1 } else { 0 };
        if !monitor.enabled {
            monitor_string += &format!("keyword monitor {},disabled;", monitor.name);
        } else {
            monitor_string += &format!(
                "monitor={},{}x{}@{},{}x{},{:.6},transform,{},vrr,{}\n",
                monitor.name,
                monitor.size.0,
                monitor.size.1,
                monitor.refresh_rate,
                monitor.offset.0,
                monitor.offset.1,
                monitor.scale,
                monitor.transform,
                vrr
            );
        }
    }

    input_config
        .write_all(monitor_string.as_bytes())
        .expect("Failed to write to file");
    input_config.sync_all().expect("Failed to sync file");
}

fn get_json() -> Result<std::process::Output, std::io::Error> {
    if is_flatpak() {
        Command::new("flatpak-spawn")
            .args(["--host", "hyprctl", "monitors", "-j"])
            .output()
    } else {
        Command::new("hyprctl").args(["-j", "monitors"]).output()
    }
}

#[allow(non_snake_case)]
#[derive(serde::Serialize, serde::Deserialize, Debug, Default)]
pub struct HyprMonitor {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub width: i64,
    pub height: i64,
    pub refreshRate: f64,
    pub x: i64,
    pub y: i64,
    pub scale: f64,
    pub transform: i64,
    pub vrr: bool,
    pub activelyTearing: bool,
    pub disabled: bool,
    pub availableModes: Vec<String>,
}

impl HyprMonitor {
    pub fn convert_to_regular_monitor(self) -> Monitor {
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
            self.vrr,
            false,
            self.x as i32,
            self.y as i32,
            self.width as i32,
            self.height as i32,
            string_to_modes(self.availableModes),
            false,
            HYPRFEATURES,
        )
    }
}

fn monitor_to_configstring(monitors: &Vec<Monitor>) -> String {
    let mut strings = Vec::new();

    for monitor in monitors {
        // re-enabled when switching is supported on the fly
        // let vrr = if monitor.vrr { 1 } else { 0 };
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
                &monitor.transform,
                // vrr
            ));
        }
    }

    strings.concat()
}

fn string_to_modes(available_modes: Vec<String>) -> Vec<AvailableMode> {
    let mut converted_modes = Vec::new();
    let mut resolutions: HashMap<&str, HashSet<(u32, String)>> = HashMap::new();
    for mode in available_modes.iter() {
        let (resolution, refresh_rate) = mode.split_once('@').unwrap();
        let entry = resolutions.get_mut(resolution);
        if let Some(entry) = entry {
            let float_hz: f64 = refresh_rate.strip_suffix("Hz").unwrap().parse().unwrap();
            let refresh_rate: u32 = float_hz.round() as u32;
            entry.insert((refresh_rate, "".into()));
            continue;
        }
        resolutions.insert(resolution, HashSet::new());
        let entry = resolutions.get_mut(resolution).unwrap();
        let float_hz: f64 = refresh_rate.strip_suffix("Hz").unwrap().parse().unwrap();
        let refresh_rates: u32 = float_hz.round() as u32;
        entry.insert((refresh_rates, "".into()));
    }
    for (resolution, refresh_rates) in resolutions {
        let (resolution_x, resolution_y) = resolution.split_once('x').unwrap();
        let mut refresh_rates: Vec<(u32, String)> = refresh_rates.into_iter().collect();
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
        if a.size < b.size {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });
    converted_modes
}
