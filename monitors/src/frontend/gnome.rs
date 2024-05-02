use std::{cell::RefCell, rc::Rc};

use adw::{
    prelude::ComboRowExt,
    prelude::{PreferencesGroupExt, PreferencesRowExt},
    PreferencesGroup,
};
use gtk::{prelude::WidgetExt, StringList};

use crate::utils::Monitor;

pub fn g_add_scaling_adjustment(
    scale: f64,
    monitor_index: usize,
    scaling_ref: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    let mut selected_scale = 0;
    let mut model = StringList::new(&["1.0"]);
    {
        let monitors = scaling_ref.borrow();
        let monitor = monitors.get(monitor_index).unwrap();
        for mode in monitor.available_modes.iter() {
            if mode.id == monitor.mode {
                let mut scales = Vec::new();
                for (i, val) in mode.supported_scales.iter().enumerate() {
                    if scale == *val {
                        selected_scale = i;
                    }
                    scales.push(val.to_string());
                }
                let mut scales: Vec<&str> = scales.iter().map(|val| val.as_str()).collect();
                scales.sort_unstable();
                model = gtk::StringList::new(&scales);
                break;
            }
        }
    }
    let scaling = adw::ComboRow::new();
    scaling.set_model(Some(&model));
    scaling.set_title("Scaling");
    scaling.set_selected(selected_scale as u32);
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
