use std::path::PathBuf;

use once_cell::sync::Lazy;
use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

pub const BASE: &str = "org.Xetibo.ReSet.Daemon";
pub const DBUS_PATH: &str = "/org/Xebito/ReSet/Plugins/Keyboard";
pub const INTERFACE: &str = "org.Xetibo.ReSet.Keyboard";

pub const HYPRLAND: &str = "Hyprland";
pub const GNOME: &str = "GNOME";
pub const KDE: &str = "KDE";

pub static HYPRLAND_DEFAULT_FILE: Lazy<PathBuf> = Lazy::new(|| {
    let base = xdg::BaseDirectories::new();
    if let Err(_error) = base {
        ERROR!(
            format!("Could not get xdg_config_home: {}", _error),
            ErrorLevel::Critical
        );
        return PathBuf::from("");
    }
    let base = base.unwrap();
    let file = base.get_config_home().join("reset/keyboard.conf");
    if let Some(path) = base.find_config_file(&file) {
        path
    } else {
        base.place_config_file(file).unwrap_or(PathBuf::from(""))
    }
});
