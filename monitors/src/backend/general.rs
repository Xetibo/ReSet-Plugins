// This file handles general functions for monitor conversions

use std::collections::HashMap;

use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};
use wayland_client::backend::ObjectId;
use wayland_protocols_plasma::output_device::v2::client::kde_output_device_mode_v2::KdeOutputDeviceModeV2;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

use crate::utils::{get_environment, Monitor};

use super::{
    gnome::g_apply_monitor_config,
    hyprland::{hy_apply_monitor_information, hy_save_monitor_configuration},
    kde::{kde_apply_monitor_config, kde_save_monitor_config},
    kwin::kwin_apply_monitor_configuration,
    utils::get_wl_backend,
    wlr::wlr_apply_monitor_configuration,
};

// temporary application of configuration
pub fn apply_monitor_configuration(
    monitors: &Vec<Monitor>,
    kwin_modes: &[HashMap<u32, KdeOutputDeviceModeV2>],
    wlr_modes: &[HashMap<u32, ZwlrOutputModeV1>],
) {
    match get_environment().as_str() {
        "Hyprland" => hy_apply_monitor_information(monitors),
        "GNOME" => g_apply_monitor_config(1, monitors),
        //"KDE" => kde_apply_monitor_config(monitors),
        "KDE" => kwin_apply_monitor_configuration(monitors, kwin_modes),
        // fallback to protocol implementations
        _ => match get_wl_backend().as_str() {
            "WLR" => wlr_apply_monitor_configuration(monitors, wlr_modes),
            "KWIN" => kwin_apply_monitor_configuration(monitors, kwin_modes),
            _ => ERROR!("Unsupported Environment", ErrorLevel::PartialBreakage),
        },
    };
}

// persistent application of configuration
pub fn save_monitor_configuration(
    monitors: &Vec<Monitor>,
    kwin_modes: &[HashMap<u32, KdeOutputDeviceModeV2>],
    wlr_modes: &[HashMap<u32, ZwlrOutputModeV1>],
) {
    match get_environment().as_str() {
        "Hyprland" => hy_save_monitor_configuration(monitors),
        "GNOME" => g_apply_monitor_config(2, monitors),
        "KDE" => kde_save_monitor_config(monitors),
        _ => match get_wl_backend().as_str() {
            "KWIN" => kwin_apply_monitor_configuration(monitors, kwin_modes),
            _ => ERROR!("Unsupported Environment", ErrorLevel::PartialBreakage),
        },
    };
}
