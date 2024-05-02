use std::{cell::RefCell, rc::Rc};

use adw::{
    prelude::ComboRowExt,
    prelude::{PreferencesGroupExt, PreferencesRowExt},
    PreferencesGroup,
};
use gtk::{prelude::WidgetExt, StringList};

use crate::utils::Monitor;

pub fn g_add_scaling_adjustment(
    _: f64,
    monitor_index: usize,
    scaling_ref: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    let mut model = StringList::new(&[""]);
    {
        let monitors = scaling_ref.borrow();
        let monitor = monitors.get(monitor_index).unwrap();
        for mode in monitor.available_modes.iter() {
            if mode.id == monitor.mode {
                let scales: Vec<String> = mode
                    .supported_scales
                    .iter()
                    .map(|val| val.to_string())
                    .collect();
                let scales: Vec<&str> = scales.iter().map(|val| val.as_str()).collect();
                model = gtk::StringList::new(&scales);
                break;
            }
        }
    }
    let scaling = adw::ComboRow::new();
    scaling.set_model(Some(&model));
    scaling.set_title("Scaling");
    scaling.connect_selected_item_notify(move |dropdown| {
        let index = dropdown.selected();
        let mut monitors = scaling_ref.borrow_mut();
        let monitor = monitors.get_mut(monitor_index).unwrap();
        for mode in monitor.available_modes.iter() {
            if mode.id == monitor.mode {
                monitor.scale = *mode.supported_scales.get(index as usize).unwrap();
                break;
            }
        }
        dropdown
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
    });
    settings.add(&scaling);
}
