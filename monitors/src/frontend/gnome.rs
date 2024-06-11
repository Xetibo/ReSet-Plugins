use std::{cell::RefCell, rc::Rc};

use adw::{
    prelude::ComboRowExt,
    prelude::{PreferencesGroupExt, PreferencesRowExt},
    PreferencesGroup,
};
use gtk::{prelude::WidgetExt, DrawingArea, StringList};

use crate::utils::Monitor;

use super::handlers::rearrange_monitors;

pub fn g_add_scaling_adjustment(
    scale: f64,
    monitor_index: usize,
    scaling_ref: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
    drawing_area: DrawingArea,
) {
    let mut selected_scale = 0;
    let mut model = StringList::new(&["100%"]);
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
                    // NOTE: GTK doesn't allow to display the number 1 as text, workaround by
                    // showing scaling for Gnome in percentages
                    scales.push(((val * 100.0) as i32).to_string() + "%");
                }
                let scales: Vec<&str> = scales.iter().map(|val| val.as_str()).collect();
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
        let mut original_monitor = None;
        for mode in monitor.available_modes.iter() {
            if mode.id == monitor.mode {
                original_monitor = Some(monitor.clone());
                monitor.scale = *mode.supported_scales.get(index as usize).unwrap();
                break;
            }
        }
        if let Some(original_monitor) = original_monitor {
            rearrange_monitors(original_monitor, monitors);
        }
        drawing_area.queue_draw();
        dropdown
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
    });
    settings.add(&scaling);
}
