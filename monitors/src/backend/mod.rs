use std::sync::{Arc, RwLock, RwLockWriteGuard};

use dbus_crossroads::IfaceBuilder;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};
use re_set_lib::{
    utils::{plugin::PluginTestFunc, plugin_setup::CrossWrapper},
    ERROR,
};

use crate::utils::{get_environment, Monitor, MonitorData};

use self::{
    general::{apply_monitor_configuration, save_monitor_configuration},
    gnome::g_get_monitor_information,
    hyprland::hy_get_monitor_information,
    kde::kde_get_monitor_information,
    kwin::kwin_get_monitor_information,
    utils::get_wl_backend,
    wlr::wlr_get_monitor_information,
};

pub mod general;
pub mod gnome;
pub mod hyprland;
pub mod kde;
pub mod kwin;
pub mod utils;
pub mod wlr;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_tests() -> Vec<PluginTestFunc> {
    vec![]
}

//backend
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn name() -> String {
    String::from("Monitors")
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn dbus_interface(cross: Arc<RwLock<CrossWrapper>>) {
    let mut cross = cross.write().unwrap();
    let interface = setup_dbus_interface(&mut cross);
    let env = get_environment();
    let mut data = MonitorData {
        monitors: match env.as_str() {
            "Hyprland" => hy_get_monitor_information(),
            "GNOME" => g_get_monitor_information(),
            //"KDE" => kde_get_monitor_information(),
            "KDE" => kwin_get_monitor_information(),
            // fallback to protocol implementations
            _ => match get_wl_backend().as_str() {
                "WLR" => wlr_get_monitor_information(),
                "KWIN" => kwin_get_monitor_information(),
                _ => {
                    ERROR!("Unsupported Environment", ErrorLevel::PartialBreakage);
                    Vec::new()
                }
            },
        },
        wl_object_ids: Vec::new(),
    };
    for monitor in data.monitors.iter() {
        data.wl_object_ids.push(monitor.wl_object_ids.clone());
    }
    if data.monitors.is_empty() {
        // means the environment is not supported
        // hence don't show the plugin
        return;
    }
    cross.insert::<MonitorData>("Monitors", &[interface], data);
}

#[no_mangle]
pub extern "C" fn backend_startup() {}

#[no_mangle]
pub extern "C" fn backend_shutdown() {}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn backend_tests() -> Vec<PluginTestFunc> {
    Vec::new()
}

pub fn setup_dbus_interface(
    cross: &mut RwLockWriteGuard<CrossWrapper>,
) -> dbus_crossroads::IfaceToken<MonitorData> {
    cross.register::<MonitorData>(
        "org.Xetibo.ReSet.Monitors",
        |c: &mut IfaceBuilder<MonitorData>| {
            c.method(
                "GetMonitors",
                (),
                ("monitors",),
                move |_, d: &mut MonitorData, ()| Ok((d.monitors.clone(),)),
            );
            c.method(
                "SetMonitors",
                ("monitors",),
                (),
                move |_, d: &mut MonitorData, (monitors,): (Vec<Monitor>,)| {
                    apply_monitor_configuration(&monitors, &d.wl_object_ids);
                    d.monitors = monitors;
                    Ok(())
                },
            );
            c.method(
                "SaveMonitors",
                ("monitors",),
                (),
                move |_, d: &mut MonitorData, (monitors,): (Vec<Monitor>,)| {
                    save_monitor_configuration(&monitors);
                    d.monitors = monitors;
                    Ok(())
                },
            );
        },
    )
}
