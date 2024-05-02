use std::{cell::RefCell, rc::Rc};

use adw::{prelude::PreferencesGroupExt, prelude::PreferencesRowExt, PreferencesGroup};

use crate::utils::Monitor;

use super::scaling_update;

pub fn g_add_scaling_adjustment(
    scale: f64,
    monitor_index: usize,
    scaling_ref: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    let scaling = adw::ComboRow::new();
    // let scaling_adjustment = gtk::Adjustment::new(scale, 0.1, 10.0, 0.15, 0.0, 0.0);
    // let scaling = adw::SpinRow::new(Some(&scaling_adjustment), 0.000001, 2);
    // scaling.set_title("Scaling");
    // scaling.connect_value_notify(move |state| {
    //     scaling_update(state, scaling_ref.clone(), monitor_index);
    // });
    settings.add(&scaling);
}
