use crate::r#const::{BASE, DBUS_PATH, INTERFACE};
use dbus::blocking::Connection;
use dbus::Error;
use std::process::Output;
use std::time::Duration;

pub fn parse_setting(command_output: Output) -> Vec<String> {
    let output = String::from_utf8(command_output.stdout).expect("not utf8");
    let output = output.lines().next().unwrap();
    let output = output.replace("str:", "");
    let output: Vec<String> = output.split(",").map(|s| s.to_string()).collect();
    output
}

pub fn get_default_path() -> String {
    let dirs = directories_next::ProjectDirs::from("org", "Xetibo", "ReSet").unwrap();
    let buf = dirs.config_dir().join("keyboard.conf");
    let path = buf.to_str().unwrap();
    String::from(path)
}

pub fn get_environment() -> String {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP");
    if desktop.is_err() {
        return "NONE".into();
    }
    desktop.unwrap()
}

pub fn get_max_active_keyboards() -> u32 {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(u32,), Error> = proxy.method_call(INTERFACE, "GetMaxActiveKeyboards", ());
    if res.is_err() {
        return 0;
    }
    res.unwrap().0
}

