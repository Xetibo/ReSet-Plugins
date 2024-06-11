use crate::utils::{GNOME, HYPRLAND, KDE};

pub const BASE: &str = "org.Xetibo.ReSet.Daemon";
pub const DBUS_PATH: &str = "/org/Xebito/ReSet/Plugins/Monitors";
pub const INTERFACE: &str = "org.Xetibo.ReSet.Monitors";

pub const SUPPORTED_ENVIRONMENTS: [&str; 5] = [HYPRLAND, GNOME, "ubuntu:GNOME", "pop:GNOME", KDE];
