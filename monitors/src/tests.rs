use crate::{
    backend::{
        gnome::{gnome_features, GnomeLogicalMonitor, GnomeMonitor, GnomeMonitorConfig},
        hyprland::{HyprMonitor, HYPRFEATURES},
        kde::{KDEMode, KDEMonitor, KDE_FEATURES},
    },
    utils::{AvailableMode, Monitor, Offset, Size},
};

#[test]
fn single_overlap() {
    let monitor = Monitor {
        size: Size(100, 100),
        offset: Offset(50, 50),
        ..Default::default()
    };
    assert!(monitor.intersect_vertical(0, 100));
    assert!(!monitor.intersect_horizontal(0, 50));
}

#[test]
fn double_overlap() {
    let monitor = Monitor {
        size: Size(100, 100),
        offset: Offset(50, 50),
        ..Default::default()
    };
    assert!(monitor.intersect_vertical(0, 100));
    assert!(monitor.intersect_horizontal(0, 100));
}

#[test]
fn no_overlap() {
    let monitor = Monitor {
        size: Size(100, 100),
        offset: Offset(50, 50),
        ..Default::default()
    };
    assert!(!monitor.intersect_vertical(0, 50));
    assert!(!monitor.intersect_horizontal(0, 50));
}

#[test]
fn convert_hyprmonitor() {
    let hypr_monitor = HyprMonitor {
        ..Default::default()
    };
    let monitor = Monitor {
        // hyprland has disabled instead -> invert
        enabled: true,
        features: HYPRFEATURES,
        ..Default::default()
    };
    assert_eq!(monitor, hypr_monitor.convert_to_regular_monitor());
}

#[test]
fn convert_gnomemonitor() {
    let gnome_monitor = GnomeMonitor {
        ..Default::default()
    };
    let logical_gnome_monitor = GnomeLogicalMonitor {
        ..Default::default()
    };
    let gnome_monitor_config = GnomeMonitorConfig {
        monitors: vec![gnome_monitor],
        logical_monitors: vec![logical_gnome_monitor],
        ..Default::default()
    };
    let monitor = Monitor {
        // hyprland has disabled instead -> invert
        enabled: false,
        mode: String::from("-1"),
        features: gnome_features(false),
        ..Default::default()
    };
    assert_eq!(
        monitor,
        gnome_monitor_config
            .inplace_to_regular_monitor()
            .pop()
            .unwrap()
    );
}

#[test]
fn convert_kde_monitor() {
    let mode = KDEMode {
        ..Default::default()
    };
    let kde_monitor = KDEMonitor {
        modes: vec![mode],
        ..Default::default()
    };
    let monitor = Monitor {
        enabled: false,
        //mode: String::from("-1"),
        available_modes: vec![AvailableMode {
            id: String::from(""),
            size: Size(0, 0),
            refresh_rates: vec![0],
            supported_scales: Vec::new(),
        }],
        features: KDE_FEATURES,
        ..Default::default()
    };
    assert_eq!(monitor, kde_monitor.convert_to_regular_monitor());
}
