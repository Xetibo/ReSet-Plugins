use std::sync::{Arc, RwLock, RwLockWriteGuard};

use dbus_crossroads::IfaceBuilder;
use re_set_lib::utils::{plugin::PluginTestFunc, plugin_setup::CrossWrapper};

use crate::{
    backend::hyprland::hy_get_monitor_information,
    utils::{get_environment, Monitor, MonitorData},
};

use self::hyprland::{apply_monitor_configuration, save_monitor_configuration};

pub mod hyprland;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_tests() -> Vec<PluginTestFunc> {
    println!("frontend tests called");
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
    match get_environment().as_str() {
        "Hyprland" => cross.insert::<MonitorData>(
            "Monitors",
            &[interface],
            MonitorData {
                monitors: hy_get_monitor_information(),
            },
        ),
        _ => println!("Environment not supported!"),
    };
}

#[no_mangle]
pub extern "C" fn backend_startup() {
    println!("startup called");
}

#[no_mangle]
pub extern "C" fn backend_shutdown() {
    println!("shutdown called");
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn backend_tests() -> Vec<PluginTestFunc> {
    println!("tests called");
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
                    apply_monitor_configuration(&monitors);
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
