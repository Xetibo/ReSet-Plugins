use std::{cell::RefCell, rc::Rc};

use adw::{prelude::PreferencesGroupExt, prelude::PreferencesRowExt, PreferencesGroup};
use gtk::{
    prelude::BoxExt,
    prelude::{ButtonExt, WidgetExt},
    DrawingArea,
};

use crate::utils::{get_environment, Monitor, GNOME};

use super::handlers::{apply_monitor_clicked, scaling_update};

pub fn arbitrary_add_scaling_adjustment(
    scale: f64,
    monitor_index: usize,
    monitors: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
    drawing_area: DrawingArea,
) {
    let scaling_adjustment = gtk::Adjustment::new(scale, 0.1, 4.0, 0.05, 0.0, 0.0);
    let scaling = adw::SpinRow::new(Some(&scaling_adjustment), 0.000001, 2);
    scaling.set_tooltip_markup(Some("This allows you to set your own custom scale.\nPlease note, that the scale needs to result in a full number for both width and height of the resolution."));
    scaling.set_title("Scaling");
    scaling.connect_value_notify(move |state| {
        scaling_update(state, monitors.clone(), monitor_index, drawing_area.clone());
    });

    settings.add(&scaling);
}

pub fn add_save_button(
    save_ref: Rc<RefCell<Vec<Monitor>>>,
    fallback_save_ref: Rc<RefCell<Vec<Monitor>>>,
    settings_box_ref_save: gtk::Box,
    drawing_ref_save: DrawingArea,
    apply_row: gtk::Box,
) -> Option<gtk::Button> {
    let mut save = None;
    match get_environment().as_str() {
        GNOME | "ubuntu:GNOME" | "Hyprland" => {
            let button = gtk::Button::builder()
                .label("Save")
                .hexpand_set(false)
                .halign(gtk::Align::End)
                .sensitive(false)
                .build();
            button.set_tooltip_markup(Some("Persistant saving of configuration"));
            button.connect_clicked(move |_| {
                apply_monitor_clicked(
                    save_ref.clone(),
                    fallback_save_ref.clone(),
                    &settings_box_ref_save,
                    &drawing_ref_save,
                    false,
                    true,
                );
            });
            apply_row.append(&button);
            save = Some(button);
        }
        _ => (),
    }
    save
}

pub fn add_primary_monitor_option(
    monitor_index: usize,
    monitors: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    let primary_value;
    {
        let monitor = monitors.borrow();
        let monitor = monitor.get(monitor_index).unwrap();
        if !monitor.features.primary {
            return;
        }
        primary_value = monitor.primary;
    }

    let primary = adw::SwitchRow::new();
    primary.set_title("Primary Monitor");
    primary.set_active(primary_value);
    primary.set_tooltip_markup(Some("Changes your primary monitor"));

    if monitors.borrow().len() < 2 {
        return;
    }
    primary.connect_active_notify(move |state| {
        for (i, monitor) in monitors.borrow_mut().iter_mut().enumerate() {
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

pub fn add_vrr_monitor_option(
    monitor_index: usize,
    monitors: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    let vrr_value;
    {
        let monitor = monitors.borrow();
        let monitor = monitor.get(monitor_index).unwrap();
        if !monitor.features.vrr {
            return;
        }
        vrr_value = monitor.vrr;
    }

    let vrr = adw::SwitchRow::new();
    vrr.set_title("Variable Refresh-Rate");
    vrr.set_active(vrr_value);
    if get_environment().as_str() == "Hyprland" {
        vrr.set_tooltip_markup(Some("Please note that this option will set the configuration for this monitor, however, if your monitor does not offer VRR, this setting will fail to make a change."));
    } else {
        vrr.set_tooltip_markup(Some("Enable or disable Variable Refresh Rate"));
    }
    vrr.connect_active_notify(move |state| {
        monitors.borrow_mut().get_mut(monitor_index).unwrap().vrr = state.is_active();
        state
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
    });
    settings.add(&vrr);
}

pub fn add_enabled_monitor_option(
    monitor_index: usize,
    monitors_ref: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    let monitors = monitors_ref.borrow();
    let monitor = monitors.get(monitor_index).unwrap();

    if monitors.len() < 2 {
        let title = adw::ActionRow::builder()
            .title(&monitor.name)
            .subtitle(&monitor.make)
            .build();
        settings.add(&title);
        return;
    }
    let enabled = adw::SwitchRow::builder()
        .title(&monitor.name)
        .subtitle(&monitor.make)
        .active(monitor.enabled)
        .build();
    enabled.set_tooltip_markup(Some("Disables or enables monitors"));
    let enabled_ref = monitors_ref.clone();
    enabled.connect_active_notify(move |state| {
        enabled_ref
            .borrow_mut()
            .get_mut(monitor_index)
            .unwrap()
            .enabled = state.is_active();
        state
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
    });
    settings.add(&enabled);
}
