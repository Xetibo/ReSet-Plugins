use std::{cell::RefCell, rc::Rc};

use adw::{prelude::PreferencesGroupExt, prelude::PreferencesRowExt, PreferencesGroup};
use gtk::prelude::WidgetExt;

use crate::utils::Monitor;

use super::scaling_update;

pub fn arbitrary_add_scaling_adjustment(
    scale: f64,
    monitor_index: usize,
    monitors: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    let scaling_adjustment = gtk::Adjustment::new(scale, 0.1, 10.0, 0.15, 0.0, 0.0);
    let scaling = adw::SpinRow::new(Some(&scaling_adjustment), 0.000001, 2);
    scaling.set_title("Scaling");
    scaling.connect_value_notify(move |state| {
        scaling_update(state, monitors.clone(), monitor_index);
    });
    settings.add(&scaling);
}

pub fn add_primary_monitor_option_generic(
    monitor_index: usize,
    monitors: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    let monitor = monitors.borrow();
    let monitor = monitor.get(monitor_index).unwrap();
    let primary = adw::SwitchRow::new();

    primary.set_title("Primary Monitor");
    primary.set_active(monitor.primary);
    let primary_ref = monitors.clone();
    primary.connect_active_notify(move |state| {
        for (i, monitor) in primary_ref.borrow_mut().iter_mut().enumerate() {
            if i == monitor_index {
                monitor.primary = state.is_active();
            } else {
                monitor.primary = !state.is_active();
            }
        }
        state
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
    });

    settings.add(&primary);
}
