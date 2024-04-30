// This file handles general functions for monitor conversions

use crate::utils::{get_environment, Monitor};

use super::{gnome::g_apply_monitor_config, hyprland::{hy_apply_monitor_information, hy_save_monitor_configuration}};

// temporary application of configuration
pub fn apply_monitor_configuration(monitors: &Vec<Monitor>) {
    match get_environment().as_str() {
        "Hyprland" => hy_apply_monitor_information(monitors),
        "GNOME" => g_apply_monitor_config(1, monitors),
        _ => println!("Environment not supported!"),
    };
}

// persistent application of configuration
pub fn save_monitor_configuration(monitors: &Vec<Monitor>) {
    match get_environment().as_str() {
        "Hyprland" => hy_save_monitor_configuration(monitors),
        "GNOME" => g_apply_monitor_config(2, monitors),
        _ => println!("Environment not supported!"),
    };
}
