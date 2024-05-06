use std::{
    cell::RefCell,
    f64::consts,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use adw::{
    prelude::{
        ActionRowExt, AdwDialogExt, AlertDialogExt, AlertDialogExtManual, ComboRowExt,
        PreferencesGroupExt, PreferencesRowExt,
    },
    PreferencesGroup, SpinRow,
};
use dbus::{blocking::Connection, Error};
use glib::object::CastNone;
use gtk::{
    gdk::prelude::SurfaceExt,
    gio,
    prelude::{BoxExt, DrawingAreaExtManual, GdkCairoContextExt, NativeExt, WidgetExt},
    DrawingArea, StringList, StringObject,
};
use re_set_lib::{
    utils::{config::get_config_value, macros::ErrorLevel},
    write_log_to_file, ERROR,
};

use crate::{
    r#const::{BASE, DBUS_PATH, INTERFACE},
    utils::{
        get_environment, get_monitor_data, Monitor, SnapDirectionHorizontal, SnapDirectionVertical,
        Wrapper,
    },
};

use super::{
    general::{
        add_primary_monitor_option, add_vrr_monitor_option, arbitrary_add_scaling_adjustment,
    },
    gnome::g_add_scaling_adjustment,
};

pub fn apply_monitor_clicked(
    apply_ref: Rc<RefCell<Vec<Monitor>>>,
    fallback: Rc<RefCell<Vec<Monitor>>>,
    settings_box_ref_apply: &gtk::Box,
    drawing_ref_apply: &DrawingArea,
    revert: bool,
) {
    let previous_state = Arc::new(AtomicBool::new(false));
    let previous_state_ref = previous_state.clone();
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(), Error> = if revert {
        proxy.method_call(INTERFACE, "SetMonitors", (fallback.borrow().clone(),))
    } else {
        proxy.method_call(INTERFACE, "SetMonitors", (apply_ref.borrow().clone(),))
    };
    if res.is_err() {
        ERROR!(
            "Could not apply monitor configuration",
            ErrorLevel::Recoverable
        );
    }
    if let Some(child) = settings_box_ref_apply.first_child() {
        settings_box_ref_apply.remove(&child);
    }
    let mut index = 0;
    for (i, monitor) in apply_ref.borrow_mut().iter_mut().enumerate() {
        if monitor.drag_information.clicked {
            index = i;
        };
    }
    apply_ref.replace(get_monitor_data());
    settings_box_ref_apply.append(&get_monitor_settings_group(apply_ref.clone(), index));
    drawing_ref_apply.queue_draw();
    drawing_ref_apply
        .activate_action(
            "monitor.reset_monitor_buttons",
            Some(&glib::Variant::from(false)),
        )
        .expect("Could not execute reset action");

    if !revert {
        let popup = adw::AlertDialog::new(Some("Confirm Configuration"), Some("Is this configuration correct?\n
         Please confirm if this is the case, otherwise the configuration will automatically be reverted."));
        popup.add_responses(&[("revert", "Revert"), ("confirm", "Confirm")]);
        popup.set_response_appearance("revert", adw::ResponseAppearance::Destructive);
        popup.set_default_response(Some("revert"));
        popup.set_close_response("revert");

        let settings = settings_box_ref_apply.clone();
        popup.connect_response(Some("confirm"), |dialog, _| {
            dialog.close();
        });
        popup.connect_response(Some("revert"), move |dialog, _| {
            previous_state.store(true, Ordering::SeqCst);
            settings
                .activate_action(
                    "monitor.revert_monitors",
                    Some(&glib::Variant::from((true, false))),
                )
                .expect("Could not activate revert action");
            dialog.close();
        });

        let settings = settings_box_ref_apply.clone();
        let thread_settings = Wrapper {
            popup: popup.clone(),
        };

        gio::spawn_blocking(move || {
            thread::sleep(Duration::from_millis(5000));
            if previous_state_ref.load(Ordering::SeqCst) {
                return;
            }
            glib::spawn_future(async move {
                glib::idle_add_once(move || {
                    thread_settings.action();
                });
            });
        });

        popup.present(&settings);
    }
}

pub fn reset_monitor_clicked(
    reset_ref: Rc<RefCell<Vec<Monitor>>>,
    settings_box_ref_reset: &gtk::Box,
    drawing_ref_reset: &DrawingArea,
    button: &gtk::Button,
) {
    if let Some(child) = settings_box_ref_reset.first_child() {
        settings_box_ref_reset.remove(&child);
    }
    let mut index = 0;
    for (i, monitor) in reset_ref.borrow_mut().iter_mut().enumerate() {
        monitor.drag_information.changed = false;
        monitor.offset.0 = monitor.drag_information.origin_x;
        monitor.offset.1 = monitor.drag_information.origin_y;
        if monitor.drag_information.clicked {
            index = i;
        };
    }
    reset_ref.replace(get_monitor_data());
    settings_box_ref_reset.append(&get_monitor_settings_group(reset_ref.clone(), index));
    drawing_ref_reset.queue_draw();
    button
        .activate_action(
            "monitor.reset_monitor_buttons",
            Some(&glib::Variant::from(false)),
        )
        .expect("Could not execute reset action");
}

pub fn save_monitor_clicked(
    save_ref: Rc<RefCell<Vec<Monitor>>>,
    fallback: Rc<RefCell<Vec<Monitor>>>,
    settings_box_ref_save: &gtk::Box,
    drawing_ref_save: &DrawingArea,
    revert: bool,
) {
    let previous_state = Arc::new(AtomicBool::new(false));
    let previous_state_ref = previous_state.clone();
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(), Error> = if revert {
        proxy.method_call(INTERFACE, "SaveMonitors", (fallback.borrow().clone(),))
    } else {
        proxy.method_call(INTERFACE, "SaveMonitors", (save_ref.borrow().clone(),))
    };
    if res.is_err() {
        ERROR!(
            "Could not save monitor configuration",
            ErrorLevel::Recoverable
        );
    }
    get_config_value("Monitor", "save_warning", |value| {
        if let Some(warning) = value.as_bool() {
            if warning {
                settings_box_ref_save.activate_action(
                        "win.banner",
                        Some(&glib::Variant::from("When using hyprland, make sure to include the created file in your config to make the changes permanent." ))
                    ).expect("Could not show banner");
            }
        }
    });
    drawing_ref_save.queue_draw();

    if !revert {
        let popup = adw::AlertDialog::new(Some("Confirm Configuration"), Some("Is this configuration correct?\n
         Please confirm if this is the case, otherwise the configuration will automatically be reverted."));
        popup.add_responses(&[("revert", "Revert"), ("confirm", "Confirm")]);
        popup.set_response_appearance("revert", adw::ResponseAppearance::Destructive);
        popup.set_default_response(Some("revert"));
        popup.set_close_response("revert");

        let settings = settings_box_ref_save.clone();
        popup.connect_response(Some("confirm"), |_, _| {});
        popup.connect_response(Some("revert"), move |_, _| {
            previous_state.store(true, Ordering::SeqCst);
            settings
                .activate_action(
                    "monitor.revert_monitors",
                    Some(&glib::Variant::from((true, false))),
                )
                .expect("Could not activate revert action");
        });

        let settings = settings_box_ref_save.clone();
        let thread_settings = Wrapper {
            popup: popup.clone(),
        };

        gio::spawn_blocking(move || {
            thread::sleep(Duration::from_millis(5000));
            if previous_state_ref.load(Ordering::SeqCst) {
                return;
            }
            glib::spawn_future(async move {
                glib::idle_add_once(move || {
                    thread_settings.action();
                });
            });
        });

        popup.present(&settings);
    }
}

pub fn get_monitor_settings_group(
    clicked_monitor: Rc<RefCell<Vec<Monitor>>>,
    monitor_index: usize,
) -> PreferencesGroup {
    let settings = PreferencesGroup::new();

    let monitors = clicked_monitor.borrow();
    let monitor = monitors.get(monitor_index);
    if monitor.is_none() {
        ERROR!("Could not insert monitor settings", ErrorLevel::Critical);
        return settings;
    }
    let monitor = monitor.unwrap();

    let enabled = adw::SwitchRow::builder()
        .title(&monitor.name)
        .subtitle(&monitor.make)
        .active(monitor.enabled)
        // .css_name("enabled-row")
        .build();
    if monitors.len() < 2 {
        enabled.last_child().unwrap().set_sensitive(false);
        enabled.first_child().unwrap().set_sensitive(true);
        // enabled
        //     .first_child()
        //     .unwrap()
        //     .next_sibling()
        //     .unwrap()
        //     .set_sensitive(true)
    } else {
        enabled.set_sensitive(true);
    }
    let enabled_ref = clicked_monitor.clone();
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

    let primary_ref = clicked_monitor.clone();
    add_primary_monitor_option(monitor_index, primary_ref, &settings);

    let vrr_ref = clicked_monitor.clone();
    add_vrr_monitor_option(monitor_index, vrr_ref, &settings);

    let scaling_ref = clicked_monitor.clone();
    add_scale_adjustment(monitor.scale, monitor_index, scaling_ref, &settings);

    let model_list = StringList::new(&[
        "0",
        "90",
        "180",
        "270",
        "0-flipped",
        "90-flipped",
        "180-flipped",
        "270-flipped",
    ]);
    let transform = adw::ComboRow::new();
    transform.set_title("Transform");
    transform.set_model(Some(&model_list));
    match monitor.transform {
        0 => transform.set_selected(0),
        1 => transform.set_selected(1),
        2 => transform.set_selected(2),
        3 => transform.set_selected(3),
        4 => transform.set_selected(4),
        5 => transform.set_selected(5),
        6 => transform.set_selected(6),
        7 => transform.set_selected(7),
        _ => ERROR!("Received unexpected transform", ErrorLevel::Recoverable),
    }
    let transform_ref = clicked_monitor.clone();
    transform.connect_selected_item_notify(move |dropdown| {
        let mut monitor = transform_ref.borrow_mut();
        let monitor = monitor.get_mut(monitor_index).unwrap();
        match model_list.string(dropdown.selected()).unwrap().as_str() {
            "0" => monitor.transform = 0,
            "90" => monitor.transform = 1,
            "180" => monitor.transform = 2,
            "270" => monitor.transform = 3,
            "0-flipped" => monitor.transform = 4,
            "90-flipped" => monitor.transform = 5,
            "180-flipped" => monitor.transform = 6,
            "270-flipped" => monitor.transform = 7,
            _ => ERROR!("Received unexpected transform", ErrorLevel::Recoverable),
        };
        dropdown
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
    });
    settings.add(&transform);

    let mut resolutions = Vec::new();
    for mode in monitor.available_modes.iter() {
        resolutions.push((mode.size.0, mode.size.1));
    }

    let refresh_rate = adw::ComboRow::new();
    let refresh_rate_combo_ref = refresh_rate.clone();
    let model_list = StringList::new(&[]);
    let mut index = 0;
    for (i, (x, y)) in resolutions.into_iter().enumerate() {
        if x == monitor.size.0 && y == monitor.size.1 {
            index = i;
        }
        model_list.append(&(x.to_string() + "x" + &y.to_string()));
    }
    let resolution = adw::ComboRow::new();
    resolution.set_title("Resolution");
    resolution.set_model(Some(&model_list));
    resolution.set_selected(index as u32);
    let resolution_ref = clicked_monitor.clone();
    resolution.connect_selected_item_notify(move |dropdown| {
        let index = dropdown.selected();
        let selected = dropdown.selected_item();
        let selected = selected.and_downcast_ref::<StringObject>().unwrap();
        let selected = selected.string().to_string();
        let (x, y) = selected.split_once('x').unwrap();
        let refresh_rates;
        {
            let mut monitor = resolution_ref.borrow_mut();
            let monitor = monitor.get_mut(monitor_index).unwrap();
            refresh_rates = monitor
                .available_modes
                .get(index as usize)
                .unwrap()
                .refresh_rates
                .clone();
            let highest = refresh_rates.first().unwrap();
            monitor.refresh_rate = *highest;
            monitor.size.0 = x.parse().unwrap();
            monitor.size.1 = y.parse().unwrap();
        }

        let refresh_rates: Vec<String> = refresh_rates.iter().map(|x| x.to_string()).collect();
        let refresh_rates: Vec<&str> = refresh_rates.iter().map(|x| x.as_str()).collect();
        dbg!(&refresh_rates);
        let refresh_rate_model = StringList::new(&refresh_rates);
        refresh_rate_combo_ref.set_model(Some(&refresh_rate_model));
        refresh_rate_combo_ref.set_selected(0);
        dropdown
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
    });
    settings.add(&resolution);

    let mode = monitor.available_modes.get(index).unwrap();
    let refresh_rates = mode.refresh_rates.clone();

    let mut index = 0;
    for (i, refresh_rate) in refresh_rates.iter().enumerate() {
        if *refresh_rate == monitor.refresh_rate {
            index = i;
        }
    }

    let refresh_rates: Vec<String> = refresh_rates.iter().map(|x| x.to_string()).collect();
    let refresh_rates: Vec<&str> = refresh_rates.iter().map(|x| x.as_str()).collect();
    let refresh_rate_model = StringList::new(&refresh_rates);
    refresh_rate.set_model(Some(&refresh_rate_model));
    refresh_rate.set_title("Refresh-Rate");
    refresh_rate.set_selected(index as u32);
    let refresh_rate_ref = clicked_monitor.clone();
    refresh_rate.connect_selected_item_notify(move |dropdown| {
        refresh_rate_ref
            .borrow_mut()
            .get_mut(monitor_index)
            .unwrap()
            .refresh_rate = dropdown
            .selected_item()
            .and_downcast_ref::<StringObject>()
            .unwrap()
            .string()
            .to_string()
            .parse()
            .unwrap();
        dropdown
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
    });
    settings.add(&refresh_rate);

    settings
}

pub fn add_scale_adjustment(
    scale: f64,
    monitor_index: usize,
    scaling_ref: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
) {
    // Different environments allow differing values
    // Hyprland allows arbitrary scales, Gnome offers a set of supported scales per monitor mode
    match get_environment().as_str() {
        "Hyprland" => arbitrary_add_scaling_adjustment(scale, monitor_index, scaling_ref, settings),
        "GNOME" => g_add_scaling_adjustment(scale, monitor_index, scaling_ref, settings),
        "KDE" => arbitrary_add_scaling_adjustment(scale, monitor_index, scaling_ref, settings),
        _ => unreachable!(),
    };
}

pub fn drawing_callback(
    area: &DrawingArea,
    border_color: gtk::gdk::RGBA,
    color: gtk::gdk::RGBA,
    draggin_color: gtk::gdk::RGBA,
    clicked_color: gtk::gdk::RGBA,
    monitor_data: Rc<RefCell<Vec<Monitor>>>,
) {
    area.set_draw_func(move |area, context, _, _| {
        // get height of window and width of drawingwidget
        let native = area.native().unwrap();
        let surface = native.surface().unwrap();
        let max_height = (surface.height() / 2).min(550);
        let max_width = area.width();

        area.set_height_request(max_height);
        area.height_request();

        // logic to ensure max width and height with offsets and sized do not overflow drawing area
        let mut max_monitor_width = 0;
        let mut max_monitor_height = 0;
        let mut min_monitor_width = 0;
        let mut min_monitor_height = 0;
        for monitor in monitor_data.borrow().iter() {
            let (width, height) = monitor.handle_transform();
            let current_min_height = monitor.offset.1;
            let current_min_width = monitor.offset.0;
            let current_max_height = monitor.offset.1 + height;
            let current_max_width = monitor.offset.0 + width;
            if current_max_width > max_monitor_width {
                max_monitor_width = current_max_width;
            }
            if current_max_height > max_monitor_height {
                max_monitor_height = current_max_height;
            }
            if current_min_width < min_monitor_width {
                min_monitor_width = current_min_width;
            }
            if current_min_height < min_monitor_height {
                min_monitor_height = current_min_height;
            }
        }

        // bigger factor will be used in order to not break ratio
        let width_factor = (max_monitor_width - min_monitor_width) / max_width + 2;
        let height_factor = (max_monitor_height - min_monitor_height) / max_height + 2;
        let factor = if width_factor > height_factor {
            width_factor
        } else {
            height_factor
        };
        let width_offset =
            (max_width - (max_monitor_width / factor) - (min_monitor_width / factor)) / 2;
        let height_offset =
            (max_height - (max_monitor_height / factor) - (min_monitor_height / factor)) / 2;

        for monitor in monitor_data.borrow_mut().iter_mut() {
            // handle transform which could invert height and width
            let (width, height) = monitor.handle_transform();
            let offset_x = monitor.drag_information.drag_x + monitor.offset.0;
            let offset_y = monitor.drag_information.drag_y + monitor.offset.1;

            monitor.drag_information.width = width;
            monitor.drag_information.height = height;
            monitor.drag_information.factor = factor;
            monitor.drag_information.border_offset_x = width_offset;
            monitor.drag_information.border_offset_y = height_offset;

            let offset_x = width_offset + offset_x / factor;
            let offset_y = height_offset + offset_y / factor;
            let height = height / factor;
            let width = width / factor;

            // monitor
            let rec = gtk::gdk::Rectangle::new(offset_x + 5, offset_y + 5, width - 5, height - 5);
            if monitor.drag_information.drag_active {
                context.set_source_color(&draggin_color);
            } else if monitor.drag_information.clicked {
                context.set_source_color(&clicked_color);
            } else {
                context.set_source_color(&color);
            }
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");

            // borders
            context.set_source_color(&border_color);

            context.set_line_width(5.0);
            // top
            let rec = gtk::gdk::Rectangle::new(offset_x + 5, offset_y, width - 5, 5);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
            context.arc(
                offset_x as f64 + 7.5,
                (offset_y + height) as f64 - 2.5,
                5.0,
                // arcs are radian...
                consts::FRAC_PI_2,
                consts::PI,
            );
            context.stroke().expect("Could not fill context");

            // right
            let rec = gtk::gdk::Rectangle::new(offset_x + width, offset_y + 5, 5, height - 5);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
            context.arc(
                (offset_x + width) as f64 - 2.5,
                offset_y as f64 + 7.5,
                5.0,
                consts::PI + consts::FRAC_PI_2,
                consts::PI * 2.0,
            );
            context.stroke().expect("Could not fill context");

            // bottom
            let rec = gtk::gdk::Rectangle::new(offset_x + 5, offset_y + height, width - 5, 5);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
            context.arc(
                (offset_x + width) as f64 - 2.5,
                (offset_y + height) as f64 - 2.5,
                5.0,
                0.0,
                consts::FRAC_PI_2,
            );
            context.stroke().expect("Could not fill context");

            // left
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y + 5, 5, height - 5);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
            context.arc(
                offset_x as f64 + 7.5,
                offset_y as f64 + 7.5,
                5.0,
                consts::PI,
                consts::PI + consts::FRAC_PI_2,
            );
            context.stroke().expect("Could not fill context");

            // text
            // TODO: change to different color
            context.set_font_size((140 / factor) as f64);
            context.move_to((offset_x + 10) as f64, (offset_y + 30) as f64);
            context
                .show_text(&monitor.name.clone())
                .expect("Could not draw text");
            context.move_to((offset_x + 10) as f64, (offset_y + 60) as f64);
            context
                .show_text(&(monitor.size.0.to_string() + ":" + &monitor.size.1.to_string()))
                .expect("Could not draw text");
        }
    });
}

pub fn monitor_drag_start(
    x: f64,
    y: f64,
    start_ref: Rc<RefCell<Vec<Monitor>>>,
    settings_box_ref: &gtk::Box,
) {
    let mut iter = -1;
    for (index, monitor) in start_ref.borrow_mut().iter_mut().enumerate() {
        let x = x as i32;
        let y = y as i32;
        if monitor.is_coordinate_within(x, y) {
            monitor.drag_information.drag_active = true;
            monitor.drag_information.clicked = true;
            monitor.drag_information.origin_x = monitor.offset.0;
            monitor.drag_information.origin_y = monitor.offset.1;
            if let Some(child) = settings_box_ref.first_child() {
                settings_box_ref.remove(&child);
            }
            iter = index as i32;
            break;
        }
    }
    if iter == -1 {
        return;
    }
    settings_box_ref.append(&get_monitor_settings_group(
        start_ref.clone(),
        iter as usize,
    ));
}

pub fn monitor_drag_update(
    x: f64,
    y: f64,
    update_ref: Rc<RefCell<Vec<Monitor>>>,
    drawing_ref: &DrawingArea,
) {
    for monitor in update_ref.borrow_mut().iter_mut() {
        let x = x as i32;
        let y = y as i32;
        if monitor.drag_information.drag_active {
            monitor.drag_information.drag_x = x * monitor.drag_information.factor;
            monitor.drag_information.drag_y = y * monitor.drag_information.factor;
            break;
        }
    }
    drawing_ref.queue_draw();
}

pub fn monitor_drag_end(
    monitor_data: Rc<RefCell<Vec<Monitor>>>,
    drawing_ref_end: &DrawingArea,
    main_box_ref: &gtk::Box,
) {
    let mut changed = false;
    let mut endpoint_left: i32 = 0;
    let mut endpoint_bottom: i32 = 0;
    let mut endpoint_right: i32 = 0;
    let mut endpoint_top: i32 = 0;
    let mut previous_width: i32 = 0;
    let mut previous_height: i32 = 0;
    let mut snap_horizontal = SnapDirectionHorizontal::None;
    let mut snap_vertical = SnapDirectionVertical::None;
    let mut iter = -1;
    for (i, monitor) in monitor_data.borrow_mut().iter_mut().enumerate() {
        if monitor.drag_information.drag_active {
            if monitor.drag_information.drag_x != monitor.drag_information.origin_x
                && monitor.drag_information.drag_y != monitor.drag_information.origin_y
            {
                changed = true;
            }
            monitor.drag_information.drag_active = false;
            endpoint_bottom = monitor.offset.1 + monitor.drag_information.drag_y;
            endpoint_left = monitor.offset.0 + monitor.drag_information.drag_x;
            endpoint_right = endpoint_left + monitor.drag_information.width;
            endpoint_top = endpoint_bottom - monitor.drag_information.height;
            previous_width = monitor.drag_information.width;
            previous_height = monitor.drag_information.height;
            iter = i as i32;
            break;
        }
    }
    let mut intersected = false;
    if iter == -1 {
        return;
    }
    let iter = iter as usize;
    for (i, monitor) in monitor_data.borrow_mut().iter_mut().enumerate() {
        if i == iter {
            continue;
        }
        let endpoint_other_left = monitor.offset.0;
        let endpoint_other_bottom = monitor.offset.1;
        let endpoint_other_right = endpoint_other_left + monitor.drag_information.width;
        let endpoint_other_top = endpoint_other_bottom - monitor.drag_information.height;

        if endpoint_right.abs_diff(endpoint_other_left) < 100 {
            snap_horizontal = SnapDirectionHorizontal::RightLeft(endpoint_other_left);
        } else if endpoint_left.abs_diff(endpoint_other_right) < 100 {
            snap_horizontal = SnapDirectionHorizontal::LeftRight(endpoint_other_right);
        } else if endpoint_right.abs_diff(endpoint_other_right) < 100 {
            snap_horizontal = SnapDirectionHorizontal::RightRight(endpoint_other_right);
        } else if endpoint_left.abs_diff(endpoint_other_left) < 100 {
            snap_horizontal = SnapDirectionHorizontal::LeftLeft(endpoint_other_left);
        }

        if endpoint_top.abs_diff(endpoint_other_top) < 100 {
            snap_vertical = SnapDirectionVertical::TopTop(endpoint_other_top);
        } else if endpoint_bottom.abs_diff(endpoint_other_bottom) < 100 {
            snap_vertical = SnapDirectionVertical::BottomBottom(endpoint_other_bottom);
        } else if endpoint_top.abs_diff(endpoint_other_bottom) < 100 {
            snap_vertical = SnapDirectionVertical::TopBottom(endpoint_other_bottom);
        } else if endpoint_bottom.abs_diff(endpoint_other_top) < 100 {
            snap_vertical = SnapDirectionVertical::BottomTop(endpoint_other_top);
        }

        // both required for a real intersect
        let intersect_horizontal = monitor.intersect_horizontal(endpoint_left, previous_width);
        let intersect_vertical = monitor.intersect_vertical(endpoint_bottom, previous_height);

        // in case of an intersect, right to right/left to left snapping not allowed -> snap into intersect
        let allow_snap_horizontal = match snap_horizontal {
            SnapDirectionHorizontal::RightRight(_) => false,
            SnapDirectionHorizontal::RightLeft(_) => true,
            SnapDirectionHorizontal::LeftLeft(_) => false,
            SnapDirectionHorizontal::LeftRight(_) => true,
            SnapDirectionHorizontal::None => false,
        };
        // same here with top to top and bottom to bottom
        let allow_snap_vertical = match snap_vertical {
            SnapDirectionVertical::TopTop(_) => false,
            SnapDirectionVertical::TopBottom(_) => true,
            SnapDirectionVertical::BottomBottom(_) => false,
            SnapDirectionVertical::BottomTop(_) => true,
            SnapDirectionVertical::None => false,
        };

        if intersect_horizontal
            && intersect_vertical
            && (!allow_snap_vertical || !allow_snap_horizontal)
        {
            intersected = true;
            changed = false;
            break;
        }
    }
    let mut monitor = monitor_data.borrow_mut();
    let monitor = monitor.get_mut(iter).unwrap();
    if intersected {
        monitor.offset.0 = monitor.drag_information.origin_x;
        monitor.offset.1 = monitor.drag_information.origin_y;
        drawing_ref_end.queue_draw();
    } else {
        match snap_horizontal {
            SnapDirectionHorizontal::RightRight(snap)
            | SnapDirectionHorizontal::RightLeft(snap) => {
                monitor.offset.0 = snap - monitor.drag_information.width;
            }
            SnapDirectionHorizontal::LeftRight(snap) | SnapDirectionHorizontal::LeftLeft(snap) => {
                monitor.offset.0 = snap;
            }
            SnapDirectionHorizontal::None => monitor.offset.0 += monitor.drag_information.drag_x,
        }
        match snap_vertical {
            SnapDirectionVertical::TopTop(snap) | SnapDirectionVertical::TopBottom(snap) => {
                monitor.offset.1 = snap + monitor.drag_information.height;
            }
            SnapDirectionVertical::BottomTop(snap) | SnapDirectionVertical::BottomBottom(snap) => {
                monitor.offset.1 = snap;
            }
            SnapDirectionVertical::None => monitor.offset.1 += monitor.drag_information.drag_y,
        }
    }
    monitor.drag_information.drag_x = 0;
    monitor.drag_information.drag_y = 0;

    drawing_ref_end.queue_draw();
    // refs
    if changed {
        main_box_ref
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not execute reset action");
    }
}

// derived from the Hyprland implementation, copyright Hyprwm/vaxry
pub fn scaling_update(
    state: &SpinRow,
    scaling_ref: Rc<RefCell<Vec<Monitor>>>,
    monitor_index: usize,
) {
    let mut monitor = scaling_ref.borrow_mut();
    let monitor = monitor.get_mut(monitor_index).unwrap();
    let scale = state.value();
    let direction = scale > monitor.scale;

    // value is the same as before, no need to do antyhing
    if (monitor.scale * 100.0).round() / 100.0 == scale {
        return;
    }

    // multiply scale to move at smaller increments
    let mut search_scale = (scale * 120.0).round();
    let mut found = false;

    // fractional scaling can only be done when the scale divides the resolution to a whole
    // number.
    // Example: 1080 / 1.5 -> 720. E.g. the factor 1.5 will also resolve to a whole number.
    if monitor.size.0 as f64 % scale != 0.0 && monitor.size.1 as f64 % scale != 0.0 && scale != 1.0
    {
        // search the traveled distance for a possible match
        search_nearest_scale(18, &mut search_scale, monitor, direction, &mut found, false);
        // search additional distance if no match has been found
        if !found {
            search_nearest_scale(100, &mut search_scale, monitor, direction, &mut found, true);
        }

        // user has entered a scale without a possible scale nearby, show error banner
        if !found {
            state
                .activate_action(
                    "monitor.reset_monitor_buttons",
                    Some(&glib::Variant::from(false)),
                )
                .expect("Could not activate reset action");
            state
                    .activate_action(
                        "win.banner",
                        Some(&glib::Variant::from(
                            "Could not find a scale near this value which divides the resolution to a whole number.",
                        )),
                    )
                    .expect("Could not show banner");
            monitor.drag_information.prev_scale = scale;
            return;
        }

        if found {
            let search_scale = (search_scale * 100000.0).round() / 100000.0;
            monitor.scale = search_scale;
            monitor.drag_information.prev_scale = search_scale;
            state.set_value((search_scale * 100.0).round() / 100.0);
        }
    } else {
        monitor.scale = scale;
        monitor.drag_information.prev_scale = scale;
    }
    state
        .activate_action(
            "monitor.reset_monitor_buttons",
            Some(&glib::Variant::from(true)),
        )
        .expect("Could not activate reset action");
}

fn search_nearest_scale(
    amount: usize,
    search_scale: &mut f64,
    monitor: &Monitor,
    direction: bool,
    found: &mut bool,
    reverse: bool,
) {
    // reverse x for the second run
    let reverse_scale = if reverse { 1.0 } else { -1.0 };
    for x in 1..amount {
        // increment here does not equal to increment of 1, but 1/120 of an increment
        // specified at: https://wayland.app/protocols/fractional-scale-v1
        let scale_move = if direction {
            (*search_scale - (x as f64) * reverse_scale) / 120.0
        } else {
            (*search_scale + (x as f64) * reverse_scale) / 120.0
        };

        let maybe_move_x = monitor.size.0 as f64 / scale_move;
        let maybe_move_y = monitor.size.1 as f64 / scale_move;
        if maybe_move_x == maybe_move_x.round()
            && maybe_move_y == maybe_move_y.round()
            && scale_move != monitor.scale
        {
            *search_scale = scale_move;
            *found = true;
            break;
        }
    }
}
