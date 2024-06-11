use std::{cell::RefCell, rc::Rc};

use crate::{
    backend::{
        gnome::{gnome_features, GnomeLogicalMonitor, GnomeMode, GnomeMonitor, GnomeMonitorConfig},
        hyprland::{HyprMonitor, HYPRFEATURES},
        kde::{KDEMode, KDEMonitor, KDE_FEATURES},
    },
    frontend::handlers::monitor_drag_end,
    utils::{AvailableMode, DragInformation, Monitor, Offset, Size},
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
    let gnome_mode = GnomeMode::default();
    let gnome_monitor = GnomeMonitor {
        modes: vec![gnome_mode],
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
        id: 600129007,
        enabled: false,
        size: Size(500, 500),
        offset: Offset(-550, 0),
        scale: 1.0,
        features: gnome_features(),
        available_modes: vec![AvailableMode {
            id: "".into(),
            size: Size(0, 0),
            refresh_rates: vec![(0, "".into())],
            supported_scales: vec![],
        }],
        uses_mode_id: true,
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
        rotation: 1,
        ..Default::default()
    };
    let monitor = Monitor {
        enabled: false,
        //mode: String::from("-1"),
        available_modes: vec![AvailableMode {
            id: String::from(""),
            size: Size(0, 0),
            refresh_rates: vec![(0, "".into())],
            supported_scales: Vec::new(),
        }],
        features: KDE_FEATURES,
        ..Default::default()
    };
    assert_eq!(monitor, kde_monitor.convert_to_regular_monitor());
}

#[test]
fn snap_left_to_right() {
    let monitors = create_monitor_pair(Offset(600, 0));
    let monitors = monitors.borrow();
    assert_eq!(monitors.get(1).unwrap().offset.0, 500);
    assert_eq!(monitors.get(1).unwrap().offset.1, 0);
}

#[test]
fn snap_right_to_left() {
    let monitors = create_monitor_pair(Offset(-600, 0));
    let monitors = monitors.borrow();
    assert_eq!(monitors.get(1).unwrap().offset.0, -500);
    assert_eq!(monitors.get(1).unwrap().offset.1, 0);
}

#[test]
fn snap_bottom_to_top() {
    let monitors = create_monitor_pair(Offset(200, 550));
    let monitors = monitors.borrow();
    assert_eq!(monitors.get(1).unwrap().offset.0, 200);
    assert_eq!(monitors.get(1).unwrap().offset.1, 500);
}

#[test]
fn snap_top_to_bottom() {
    let monitors = create_monitor_pair(Offset(200, -550));
    let monitors = monitors.borrow();
    assert_eq!(monitors.get(1).unwrap().offset.0, 200);
    assert_eq!(monitors.get(1).unwrap().offset.1, -500);
}

#[test]
fn snap_top_to_top() {
    // since the monitor have the same size, bottom to bottom is implied as well
    let monitors = create_monitor_pair(Offset(500, -50));
    let monitors = monitors.borrow();
    assert_eq!(monitors.get(1).unwrap().offset.0, 500);
    assert_eq!(monitors.get(1).unwrap().offset.1, 0);
}

#[test]
fn snap_right_to_right() {
    // since the monitor have the same size, left to left is implied as well
    let monitors = create_monitor_pair(Offset(50, -500));
    let monitors = monitors.borrow();
    assert_eq!(monitors.get(1).unwrap().offset.0, 0);
    assert_eq!(monitors.get(1).unwrap().offset.1, -500);
}

fn create_monitor_pair(offset: Offset) -> Rc<RefCell<Vec<Monitor>>> {
    let mut dragging_monitor = Monitor {
        id: 2,
        enabled: true,
        size: Size(500, 500),
        offset,
        scale: 1.0,
        drag_information: DragInformation {
            width: 500,
            height: 500,
            ..Default::default()
        },
        ..Default::default()
    };
    dragging_monitor.drag_information.drag_active = true;
    dragging_monitor.drag_information.clicked = true;
    let monitors = Rc::new(RefCell::new(vec![
        Monitor {
            id: 1,
            enabled: true,
            size: Size(500, 500),
            offset: Offset(0, 0),
            scale: 1.0,
            drag_information: DragInformation {
                width: 500,
                height: 500,
                ..Default::default()
            },
            ..Default::default()
        },
        dragging_monitor,
    ]));
    monitor_drag_end(monitors.clone(), None, false);
    monitors
}
