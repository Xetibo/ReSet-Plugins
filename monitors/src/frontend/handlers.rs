use std::{
    cell::{RefCell, RefMut},
    cmp,
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
        AdwDialogExt, AlertDialogExt, AlertDialogExtManual, ComboRowExt, PreferencesGroupExt,
        PreferencesRowExt,
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
use re_set_lib::{utils::config::get_config_value, ERROR};

#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::{
    backend::utils::get_wl_backend,
    r#const::{BASE, DBUS_PATH, INTERFACE},
    utils::{
        get_environment, get_monitor_data, is_gnome, AlertWrapper, Monitor,
        SnapDirectionHorizontal, SnapDirectionVertical, GNOME, HYPRLAND, KDE,
    },
};

use super::{
    general::{
        add_enabled_monitor_option, add_primary_monitor_option, add_vrr_monitor_option,
        arbitrary_add_scaling_adjustment,
    },
    gnome::{g_add_scaling_adjustment, reload_scale},
};

#[derive(Clone)]
pub enum Scale {
    Arbitrary(adw::SpinRow),
    Defined(adw::ComboRow),
}

pub fn apply_monitor_clicked(
    monitor_ref: Rc<RefCell<Vec<Monitor>>>,
    fallback: Rc<RefCell<Vec<Monitor>>>,
    settings_ref: &gtk::Box,
    drawing_ref: &DrawingArea,
    revert: bool,
    persistent: bool,
) {
    let previous_state = Arc::new(AtomicBool::new(false));
    let previous_state_ref = previous_state.clone();
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(), Error> = if revert {
        if persistent {
            proxy.method_call(INTERFACE, "SaveMonitors", (fallback.borrow().clone(),))
        } else {
            proxy.method_call(INTERFACE, "SetMonitors", (fallback.borrow().clone(),))
        }
    } else if persistent {
        proxy.method_call(INTERFACE, "SaveMonitors", (monitor_ref.borrow().clone(),))
    } else {
        proxy.method_call(INTERFACE, "SetMonitors", (monitor_ref.borrow().clone(),))
    };
    if let Err(_error) = res {
        ERROR!(
            format!("Could not apply monitor configuration {}", _error),
            ErrorLevel::Recoverable
        );
    }
    if let Some(child) = settings_ref.first_child() {
        settings_ref.remove(&child);
    }
    let mut index = 0;
    for (i, monitor) in monitor_ref.borrow_mut().iter_mut().enumerate() {
        if monitor.drag_information.clicked {
            index = i;
        };
    }
    monitor_ref.replace(get_monitor_data());
    settings_ref.append(&get_monitor_settings_group(
        monitor_ref.clone(),
        index,
        drawing_ref,
    ));
    if persistent {
        get_config_value("Monitor", "save_warning", |value| {
            if let Some(warning) = value.as_bool() {
                if warning {
                    settings_ref.activate_action(
                        "win.banner",
                        Some(&glib::Variant::from("When using hyprland, make sure to include the created file in your config to make the changes permanent." ))
                    ).expect("Could not show banner");
                }
            }
        });
    }
    drawing_ref.queue_draw();
    drawing_ref
        .activate_action(
            "monitor.reset_monitor_buttons",
            Some(&glib::Variant::from(false)),
        )
        .expect("Could not execute reset action");

    if !revert {
        // Gnome has their own popup, hence two popups would appear -> solution, disable ours
        if persistent && is_gnome() {
            return;
        }
        let popup = adw::AlertDialog::new(Some("Confirm Configuration"), Some("Is this configuration correct?\n
         Please confirm if this is the case, otherwise the configuration will automatically be reverted."));
        popup.add_responses(&[("revert", "Revert"), ("confirm", "Confirm")]);
        popup.set_response_appearance("revert", adw::ResponseAppearance::Destructive);
        popup.set_default_response(Some("revert"));
        popup.set_close_response("revert");

        let settings = settings_ref.clone();
        popup.connect_response(Some("confirm"), move |dialog, _| {
            fallback.replace(monitor_ref.borrow().clone());
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

        let settings = settings_ref.clone();
        let thread_settings = AlertWrapper {
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
    settings_box_ref_reset.append(&get_monitor_settings_group(
        reset_ref.clone(),
        index,
        drawing_ref_reset,
    ));
    drawing_ref_reset.queue_draw();
    button
        .activate_action(
            "monitor.reset_monitor_buttons",
            Some(&glib::Variant::from(false)),
        )
        .expect("Could not execute reset action");
}

pub fn get_monitor_settings_group(
    clicked_monitor: Rc<RefCell<Vec<Monitor>>>,
    monitor_index: usize,
    drawing_area: &DrawingArea,
) -> PreferencesGroup {
    let settings = PreferencesGroup::new();

    {
        let mut monitors = clicked_monitor.borrow_mut();
        let monitor = monitors.get_mut(monitor_index);
        if monitor.is_none() {
            ERROR!("Could not insert monitor settings", ErrorLevel::Critical);
            return settings;
        }
        monitor.unwrap().drag_information.clicked = true;
    }
    let monitors = clicked_monitor.borrow();
    let monitor = monitors.get(monitor_index).unwrap();

    let enabled_ref = clicked_monitor.clone();
    add_enabled_monitor_option(monitor_index, enabled_ref, &settings, drawing_area.clone());

    let primary_ref = clicked_monitor.clone();
    add_primary_monitor_option(monitor_index, primary_ref, &settings);

    let vrr_ref = clicked_monitor.clone();
    add_vrr_monitor_option(monitor_index, vrr_ref, &settings);

    let scaling_ref = clicked_monitor.clone();
    let scaling = add_scale_adjustment(
        monitor.scale,
        monitor_index,
        scaling_ref,
        &settings,
        drawing_area.clone(),
    );

    let model_list = StringList::new(&[
        "0°",
        "90°",
        "180°",
        "270°",
        "0°-flipped",
        "90°-flipped",
        "180°-flipped",
        "270°-flipped",
    ]);
    let transform = adw::ComboRow::new();
    transform.set_tooltip_markup(Some("Changes the orientation of the monitor"));
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
    let transform_drawing_ref = drawing_area.clone();
    transform.connect_selected_item_notify(move |dropdown| {
        let mut monitors = transform_ref.borrow_mut();
        let monitor = monitors.get_mut(monitor_index).unwrap();
        let original_monitor = monitor.clone();
        match model_list.string(dropdown.selected()).unwrap().as_str() {
            "0°" => monitor.transform = 0,
            "90°" => monitor.transform = 1,
            "180°" => monitor.transform = 2,
            "270°" => monitor.transform = 3,
            "0°-flipped" => monitor.transform = 4,
            "90°-flipped" => monitor.transform = 5,
            "180°-flipped" => monitor.transform = 6,
            "270°-flipped" => monitor.transform = 7,
            _ => ERROR!("Received unexpected transform", ErrorLevel::Recoverable),
        };
        rearrange_monitors(original_monitor, monitors);
        dropdown
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
        transform_drawing_ref.queue_draw();
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
    resolution.set_tooltip_markup(Some("Changes the current resolution"));
    resolution.set_title("Resolution");
    resolution.set_model(Some(&model_list));
    resolution.set_selected(index as u32);
    let resolution_ref = clicked_monitor.clone();
    let resolution_drawing_ref = drawing_area.clone();
    resolution.connect_selected_item_notify(move |dropdown| {
        let index = dropdown.selected();
        let selected = dropdown.selected_item();
        let selected = selected.and_downcast_ref::<StringObject>().unwrap();
        let selected = selected.string().to_string();
        let (x, y) = selected.split_once('x').unwrap();
        let refresh_rates;
        let uses_ids;
        {
            let mut monitors = resolution_ref.borrow_mut();
            let monitor = monitors.get_mut(monitor_index).unwrap();
            let mode = monitor.available_modes.get(index as usize).unwrap();
            refresh_rates = mode.refresh_rates.clone();
            let highest = refresh_rates.first().unwrap();
            monitor.mode = String::from(&mode.id);
            monitor.refresh_rate = highest.0;
            let new_size_x: i32 = x.parse().unwrap();
            let new_size_y: i32 = y.parse().unwrap();
            let original_monitor = monitor.clone();
            monitor.size.0 = new_size_x;
            monitor.size.1 = new_size_y;
            let (width, height) = monitor.handle_scaled_transform();
            monitor.drag_information.width = width;
            monitor.drag_information.height = height;
            uses_ids = monitor.uses_mode_id;
            rearrange_monitors(original_monitor, monitors);
        }

        let mut converted_rates = Vec::new();
        for rate in refresh_rates.into_iter() {
            let string_rate = rate.0.to_string() + "Hz";
            if uses_ids {
                if !converted_rates.contains(&string_rate) {
                    converted_rates.push(string_rate);
                }
            } else {
                converted_rates.push(string_rate);
            }
        }

        let refresh_rates: Vec<&str> = converted_rates.iter().map(|x| x.as_str()).collect();
        let refresh_rate_model = StringList::new(&refresh_rates);
        refresh_rate_combo_ref.set_model(Some(&refresh_rate_model));
        refresh_rate_combo_ref.set_selected(0);
        match scaling.clone() {
            Scale::Arbitrary(spinrow) => {
                let width;
                let height;
                let scale;
                {
                    let mut monitors = resolution_ref.borrow_mut();
                    let monitor = monitors.get_mut(monitor_index).unwrap();
                    monitor.drag_information.resolution_changed = true;
                    width = monitor.size.0;
                    height = monitor.size.1;
                    scale = monitor.scale;
                }
                let value = spinrow.value();
                if is_nonfunctional_scale(width, height, scale) {
                    // workaround to trigger a new scaling search
                    spinrow.set_value(value + 0.000001);
                }
            }
            Scale::Defined(comborow) => {
                let monitors = resolution_ref.borrow();
                let scale = monitors.get(monitor_index).unwrap().scale;
                let (model, selected_scale) = reload_scale(monitors, monitor_index, scale);
                comborow.set_selected(selected_scale);
                comborow.set_model(Some(&model));
            }
        }
        dropdown
            .activate_action(
                "monitor.reset_monitor_buttons",
                Some(&glib::Variant::from(true)),
            )
            .expect("Could not activate reset action");
        resolution_drawing_ref.queue_draw();
    });
    settings.add(&resolution);

    let mode = monitor.available_modes.get(index).unwrap();
    let refresh_rates = mode.refresh_rates.clone();
    let mut converted_rates: Vec<String> = Vec::new();

    let mut index = 0;
    let mut iter = 0;
    for refresh_rate in refresh_rates.into_iter() {
        let rate = refresh_rate.0.to_string() + "Hz";
        if monitor.uses_mode_id && converted_rates.contains(&rate) {
            // id users might see duplicate entries otherwise
            continue;
        }
        converted_rates.push(rate);
        if refresh_rate.0 == monitor.refresh_rate {
            index = iter;
        }
        iter += 1;
    }

    let refresh_rates: Vec<&str> = converted_rates.iter().map(|x| x.as_str()).collect();
    let refresh_rate_model = StringList::new(&refresh_rates);
    refresh_rate.set_model(Some(&refresh_rate_model));
    refresh_rate.set_title("Refresh-Rate");
    refresh_rate.set_tooltip_markup(Some("Changes the current refresh-rate"));
    refresh_rate.set_selected(index as u32);
    let refresh_rate_ref = clicked_monitor.clone();
    refresh_rate.connect_selected_item_notify(move |dropdown| {
        let mut monitors = refresh_rate_ref.borrow_mut();
        let monitor = monitors.get_mut(monitor_index).unwrap();
        let selected = dropdown
            .selected_item()
            .and_downcast_ref::<StringObject>()
            .unwrap()
            .string()
            .to_string()
            .trim_end_matches("Hz")
            .parse()
            .unwrap();
        if monitor.uses_mode_id {
            for mode in monitor.available_modes.iter() {
                if mode.size.0 == monitor.size.0 && mode.size.1 == monitor.size.1 {
                    for refresh_rate in mode.refresh_rates.iter() {
                        if refresh_rate.0 == selected {
                            monitor.mode = String::from(&refresh_rate.1);
                            break;
                        }
                    }
                    break;
                }
            }
        }
        monitor.refresh_rate = selected;
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

pub fn rearrange_monitors(original_monitor: Monitor, mut monitors: RefMut<'_, Vec<Monitor>>) {
    let (original_width, original_height) = original_monitor.handle_scaled_transform();
    let mut furthest = i32::MIN;
    let mut left = i32::MAX;
    let mut top = i32::MAX;
    let mut diff_x = 0;
    let mut diff_y = 0;

    // check for the difference of x or y offset
    // and set the rightmost side for overlapped monitors
    for monitor in monitors.iter_mut() {
        let is_original = monitor.id == original_monitor.id;
        if is_gnome() && ((is_original && !original_monitor.enabled) || !monitor.enabled) {
            // no need to check for monitors that are disabled on gnome -> they do not affect
            // arrangement
            continue;
        }
        // right_most for monitors that overlap -> reset monitor to available space
        let (width, height) = monitor.handle_scaled_transform();
        let right_side = monitor.offset.0 + width;
        let left_side = monitor.offset.0;
        let top_side = monitor.offset.1;
        if right_side > furthest {
            furthest = right_side;
        }
        if left_side < left {
            left = left_side;
        }
        if top_side < top {
            top = top_side;
        }

        if is_original {
            diff_x = width - original_width;
            diff_y = height - original_height;
        }
    }

    // add the difference to the rightmost side in order to not intersect
    furthest += diff_x;

    // apply offset to all affected monitors by the change
    for monitor in monitors.iter_mut() {
        let is_original = monitor.id == original_monitor.id;
        let (width, height) = monitor.handle_scaled_transform();
        if is_gnome() {
            monitor.offset.1 -= top;
            monitor.offset.0 -= left;
            if is_original && !original_monitor.enabled {
                // move previously disabled monitor to the right
                monitor.offset.0 = furthest;
                furthest = monitor.offset.0 + width;
                continue;
            }
        }
        if is_original {
            continue;
        }

        if monitor.offset.0 >= original_monitor.offset.0 + original_width {
            monitor.offset.0 += diff_x;
        }
        if monitor.offset.1 - height >= original_monitor.offset.1 {
            monitor.offset.1 += diff_y;
        }
    }

    // (false, false)
    // first: already used flag -> don't check for overlaps against the same monitors just in
    // opposite order
    // second: bool flag to indicate overlap
    let mut overlaps = vec![(false, false); monitors.len()];
    // check for overlaps
    for (index, monitor) in monitors.iter().enumerate() {
        for (other_index, other_monitor) in monitors.iter().enumerate() {
            if monitor.id == other_monitor.id || overlaps[other_index].0 {
                continue;
            }
            let (width, height) = other_monitor.handle_scaled_transform();
            let intersect_horizontal = monitor.intersect_horizontal(
                other_monitor.offset.0 + other_monitor.drag_information.border_offset_x,
                width,
            );
            let intersect_vertical = monitor.intersect_vertical(
                other_monitor.offset.1 + other_monitor.drag_information.border_offset_y,
                height,
            );
            let (width, _) = monitor.handle_scaled_transform();
            let is_furthest = furthest == monitor.offset.0 + width;
            if intersect_horizontal && intersect_vertical && !is_furthest {
                overlaps[index].1 = true;
            }
        }
        overlaps[index].0 = true;
    }

    // if overlapped, send monitor to the end -> monitor is now rightmost monitor
    for (index, monitor) in monitors.iter_mut().enumerate() {
        if overlaps[index].1 {
            let (width, _) = monitor.handle_scaled_transform();
            monitor.offset.0 = furthest;
            furthest = monitor.offset.0 + width;
        }
    }
}

pub fn add_scale_adjustment(
    scale: f64,
    monitor_index: usize,
    scaling_ref: Rc<RefCell<Vec<Monitor>>>,
    settings: &PreferencesGroup,
    drawing_area: DrawingArea,
) -> Scale {
    // Different environments allow differing values
    // Hyprland allows arbitrary scales, Gnome offers a set of supported scales per monitor mode
    match get_environment().as_str() {
        HYPRLAND => Scale::Arbitrary(arbitrary_add_scaling_adjustment(
            scale,
            monitor_index,
            scaling_ref,
            settings,
            drawing_area,
        )),
        GNOME | "ubuntu:GNOME" => Scale::Defined(g_add_scaling_adjustment(
            scale,
            monitor_index,
            scaling_ref,
            settings,
            drawing_area,
        )),
        KDE => Scale::Arbitrary(arbitrary_add_scaling_adjustment(
            scale,
            monitor_index,
            scaling_ref,
            settings,
            drawing_area,
        )),
        _ => match get_wl_backend().as_str() {
            "WLR" | "KWIN" => Scale::Arbitrary(arbitrary_add_scaling_adjustment(
                scale,
                monitor_index,
                scaling_ref,
                settings,
                drawing_area,
            )),
            _ => unreachable!(),
        },
    }
}

pub fn drawing_callback(
    area: &DrawingArea,
    border_color: gtk::gdk::RGBA,
    color: gtk::gdk::RGBA,
    draggin_color: gtk::gdk::RGBA,
    clicked_color: gtk::gdk::RGBA,
    selected_text_color: gtk::gdk::RGBA,
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
        for monitor in monitor_data.borrow_mut().iter_mut() {
            let (width, height) = monitor.handle_scaled_transform();
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
        let width_factor =
            (max_monitor_width - min_monitor_width) / max_width.clamp(0, i32::MAX) + 2;
        let height_factor =
            (max_monitor_height - min_monitor_height) / max_height.clamp(0, i32::MAX) + 2;
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
            let (width, height) = monitor.handle_scaled_transform();
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
            if monitor.drag_information.drag_active || monitor.drag_information.clicked {
                context.set_source_color(&selected_text_color);
            }
            // define text to be scaled with monitor size
            let size = (1.0 * (width / 10) as f64).clamp(12.0, 18.0);
            let gap = size;
            const TOP_GAP: f64 = 6.0;
            context.set_font_size(size);
            context.move_to((offset_x + 10) as f64, offset_y as f64 + gap + TOP_GAP);
            context
                .show_text(&monitor.name.clone())
                .expect("Could not draw text");
            context.move_to(
                (offset_x + 10) as f64,
                offset_y as f64 + gap * 2.0 + TOP_GAP,
            );
            if monitor.enabled {
                context
                    .show_text(&(monitor.size.0.to_string() + ":" + &monitor.size.1.to_string()))
                    .expect("Could not draw text");
            } else {
                context.show_text("disabled").expect("Could not draw text");
            }
            if monitor.scale != 1.0 {
                context.move_to(
                    (offset_x + 10) as f64,
                    offset_y as f64 + gap * 3.0 + TOP_GAP,
                );
                let scale = (monitor.scale * 100.0).round() / 100.0;
                context
                    .show_text(&("scale: ".to_string() + &scale.to_string()))
                    .expect("Could not draw text");
            }
        }
    });
}

pub fn monitor_drag_start(
    x: f64,
    y: f64,
    start_ref: Rc<RefCell<Vec<Monitor>>>,
    settings_box_ref: &gtk::Box,
    drawing_area: &DrawingArea,
) {
    let mut iter = -1;
    let mut previous = -1;
    {
        let mut monitors = start_ref.borrow_mut();
        for (index, monitor) in monitors.iter_mut().enumerate() {
            let x = x as i32;
            let y = y as i32;
            if monitor.is_coordinate_within(x, y) {
                if monitor.enabled {
                    monitor.drag_information.drag_active = true;
                }
                monitor.drag_information.clicked = true;
                monitor.drag_information.origin_x = monitor.offset.0;
                monitor.drag_information.origin_y = monitor.offset.1;
                if let Some(child) = settings_box_ref.first_child() {
                    settings_box_ref.remove(&child);
                }
                iter = index as i32;
            } else if monitor.drag_information.clicked {
                previous = index as i32;
            }
        }
        if iter == -1 {
            return;
        }
        if previous != -1 {
            monitors
                .get_mut(previous as usize)
                .unwrap()
                .drag_information
                .clicked = false;
        }
    }
    settings_box_ref.append(&get_monitor_settings_group(
        start_ref.clone(),
        iter as usize,
        drawing_area,
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
        if is_gnome() && !monitor.enabled {
            continue;
        }
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
    drawing_opt: Option<&DrawingArea>,
    disallow_gaps: bool,
) {
    const SNAP_DISTANCE: u32 = 150;
    let mut changed = false;
    let mut endpoint_left: i32 = 0;
    let mut endpoint_left_intersect: i32 = 0;
    let mut endpoint_bottom: i32 = 0;
    let mut endpoint_bottom_intersect: i32 = 0;
    let mut endpoint_right: i32 = 0;
    let mut endpoint_top: i32 = 0;
    let mut previous_width: i32 = 0;
    let mut previous_height: i32 = 0;
    let mut snap_horizontal = SnapDirectionHorizontal::None;
    let mut snap_vertical = SnapDirectionVertical::None;
    let mut iter = -1;
    for (i, monitor) in monitor_data.borrow_mut().iter_mut().enumerate() {
        // define values from current monitor to be compared against others
        if monitor.drag_information.drag_active {
            if monitor.drag_information.drag_x != monitor.drag_information.origin_x
                && monitor.drag_information.drag_y != monitor.drag_information.origin_y
            {
                changed = true;
            }
            monitor.drag_information.drag_active = false;
            endpoint_bottom = monitor.offset.1 + monitor.drag_information.drag_y;
            endpoint_bottom_intersect = endpoint_bottom + monitor.drag_information.border_offset_y;
            endpoint_left = monitor.offset.0 + monitor.drag_information.drag_x;
            endpoint_left_intersect = endpoint_left + monitor.drag_information.border_offset_x;
            endpoint_right = endpoint_left + monitor.drag_information.width;
            endpoint_top = endpoint_bottom + monitor.drag_information.height;
            previous_width = monitor.drag_information.width;
            previous_height = monitor.drag_information.height;
            iter = i as i32;
            break;
        }
    }
    let mut intersected = false;
    if iter == -1 {
        // clicked monitor not found, escaping
        return;
    }
    let iter = iter as usize;
    for (i, monitor) in monitor_data.borrow_mut().iter_mut().enumerate() {
        if i == iter {
            // continue if the same monitor is used -> no point in calculating
            continue;
        }
        // define other monitor endpoints
        let (other_width, other_height) = monitor.handle_scaled_transform();
        let endpoint_other_left = monitor.offset.0;
        let endpoint_other_bottom = monitor.offset.1;
        let endpoint_other_right = endpoint_other_left + monitor.drag_information.width;
        let endpoint_other_top = endpoint_other_bottom + monitor.drag_information.height;

        // find smallest difference
        let right_to_left = endpoint_right.abs_diff(endpoint_other_left);
        let left_to_right = endpoint_left.abs_diff(endpoint_other_right);
        let right_to_right = endpoint_right.abs_diff(endpoint_other_right);
        let left_to_left = endpoint_left.abs_diff(endpoint_other_left);
        let min = cmp::min(
            cmp::min(right_to_left, left_to_right),
            cmp::min(right_to_right, left_to_left),
        );

        // snap to the smallest distance if smaller than SNAP_DISTANCE
        if min < SNAP_DISTANCE {
            match min {
                x if x == right_to_left => {
                    snap_horizontal = SnapDirectionHorizontal::RightLeft(
                        endpoint_other_left,
                        endpoint_other_top,
                        other_height,
                    );
                }
                x if x == left_to_right => {
                    snap_horizontal = SnapDirectionHorizontal::LeftRight(
                        endpoint_other_right,
                        endpoint_other_top,
                        other_height,
                    );
                }
                x if x == right_to_right => {
                    snap_horizontal = SnapDirectionHorizontal::RightRight(endpoint_other_right);
                }
                x if x == left_to_left => {
                    snap_horizontal = SnapDirectionHorizontal::LeftLeft(endpoint_other_left);
                }
                _ => unreachable!(),
            }
        }

        // find smallest difference
        let top_to_bottom = endpoint_top.abs_diff(endpoint_other_bottom);
        let bottom_to_top = endpoint_bottom.abs_diff(endpoint_other_top);
        let top_to_top = endpoint_top.abs_diff(endpoint_other_top);
        let bottom_to_bottom = endpoint_bottom.abs_diff(endpoint_other_bottom);
        let min = cmp::min(
            cmp::min(top_to_bottom, bottom_to_top),
            cmp::min(top_to_top, bottom_to_bottom),
        );

        // snap to the smallest distance if smaller than SNAP_DISTANCE
        if min < SNAP_DISTANCE {
            match min {
                x if x == top_to_bottom => {
                    snap_vertical = SnapDirectionVertical::TopBottom(
                        endpoint_other_bottom,
                        endpoint_other_left,
                        other_width,
                    );
                }
                x if x == bottom_to_top => {
                    snap_vertical = SnapDirectionVertical::BottomTop(
                        endpoint_other_top,
                        endpoint_other_left,
                        other_width,
                    );
                }
                x if x == top_to_top => {
                    snap_vertical = SnapDirectionVertical::TopTop(endpoint_other_top);
                }
                x if x == bottom_to_bottom => {
                    snap_vertical = SnapDirectionVertical::BottomBottom(endpoint_other_bottom);
                }
                _ => unreachable!(),
            }
        }

        // both required for a real intersect
        let intersect_horizontal =
            monitor.intersect_horizontal(endpoint_left_intersect, previous_width);
        let intersect_vertical =
            monitor.intersect_vertical(endpoint_bottom_intersect, previous_height);

        // in case of an intersect, right to right/left to left snapping not allowed -> snap into intersect
        let allow_snap_horizontal = match snap_horizontal {
            SnapDirectionHorizontal::RightRight(_) => false,
            SnapDirectionHorizontal::RightLeft(_, _, _) => true,
            SnapDirectionHorizontal::LeftLeft(_) => false,
            SnapDirectionHorizontal::LeftRight(_, _, _) => true,
            SnapDirectionHorizontal::None => false,
        };
        // same here with top to top and bottom to bottom
        let allow_snap_vertical = match snap_vertical {
            SnapDirectionVertical::TopTop(_) => false,
            SnapDirectionVertical::TopBottom(_, _, _) => true,
            SnapDirectionVertical::BottomBottom(_) => false,
            SnapDirectionVertical::BottomTop(_, _, _) => true,
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
    let mut monitors = monitor_data.borrow_mut();
    let monitor = monitors.get_mut(iter).unwrap();
    if intersected {
        monitor.drag_information.drag_x = 0;
        monitor.drag_information.drag_y = 0;
        monitor.offset.0 = monitor.drag_information.origin_x;
        monitor.offset.1 = monitor.drag_information.origin_y;
        if let Some(drawing_area) = drawing_opt {
            drawing_area.queue_draw();
        }
        return;
    } else {
        match snap_horizontal {
            SnapDirectionHorizontal::RightRight(snap)
            | SnapDirectionHorizontal::RightLeft(snap, _, _) => {
                monitor.offset.0 = snap - monitor.drag_information.width;
            }
            SnapDirectionHorizontal::LeftRight(snap, _, _)
            | SnapDirectionHorizontal::LeftLeft(snap) => {
                monitor.offset.0 = snap;
            }
            SnapDirectionHorizontal::None => {
                // GNOME doesn't allow spacing between monitors.... why...
                if disallow_gaps
                    && match snap_vertical {
                        SnapDirectionVertical::None => true,
                        SnapDirectionVertical::TopTop(_) => true,
                        SnapDirectionVertical::BottomBottom(_) => true,
                        SnapDirectionVertical::TopBottom(_, left, width) => {
                            endpoint_left > left + width || endpoint_right < left
                        }
                        SnapDirectionVertical::BottomTop(_, left, width) => {
                            endpoint_left > left + width || endpoint_right < left
                        }
                    }
                {
                    monitor.drag_information.drag_x = 0;
                    monitor.drag_information.drag_y = 0;
                    monitor.offset.0 = monitor.drag_information.origin_x;
                    monitor.offset.1 = monitor.drag_information.origin_y;
                    if let Some(drawing_area) = drawing_opt {
                        drawing_area.queue_draw();
                    }
                    return;
                }
                monitor.offset.0 += monitor.drag_information.drag_x;
            }
        }
        match snap_vertical {
            SnapDirectionVertical::TopTop(snap) | SnapDirectionVertical::TopBottom(snap, _, _) => {
                monitor.offset.1 = snap - monitor.drag_information.height;
            }
            SnapDirectionVertical::BottomTop(snap, _, _)
            | SnapDirectionVertical::BottomBottom(snap) => {
                monitor.offset.1 = snap;
            }
            SnapDirectionVertical::None => {
                // GNOME doesn't allow spacing between monitors.... why...
                if disallow_gaps
                    && match snap_horizontal {
                        SnapDirectionHorizontal::None => true,
                        SnapDirectionHorizontal::LeftLeft(_) => true,
                        SnapDirectionHorizontal::RightRight(_) => true,
                        SnapDirectionHorizontal::RightLeft(_, top, height) => {
                            endpoint_bottom > top || endpoint_top < top - height
                        }
                        SnapDirectionHorizontal::LeftRight(_, top, height) => {
                            endpoint_bottom > top || endpoint_top < top - height
                        }
                    }
                {
                    monitor.drag_information.drag_x = 0;
                    monitor.drag_information.drag_y = 0;
                    monitor.offset.0 = monitor.drag_information.origin_x;
                    monitor.offset.1 = monitor.drag_information.origin_y;
                    if let Some(drawing_area) = drawing_opt {
                        drawing_area.queue_draw();
                    }
                    return;
                }
                monitor.offset.1 += monitor.drag_information.drag_y
            }
        }
    }
    monitor.drag_information.drag_x = 0;
    monitor.drag_information.drag_y = 0;

    if is_gnome() {
        let mut left_side = i32::MAX;
        let mut top_side = i32::MAX;
        for monitor in monitors.iter_mut() {
            if monitor.offset.0 < left_side {
                left_side = monitor.offset.0;
            }
            if monitor.offset.1 < top_side {
                top_side = monitor.offset.1;
            }
        }
        for monitor in monitors.iter_mut() {
            monitor.offset.0 -= left_side;
            monitor.offset.1 -= top_side;
        }
    }
    if let Some(drawing_area) = drawing_opt {
        drawing_area.queue_draw();
        if changed {
            drawing_area
                .activate_action(
                    "monitor.reset_monitor_buttons",
                    Some(&glib::Variant::from(true)),
                )
                .expect("Could not find action");
        }
    }
}

// derived from the Hyprland implementation
pub fn scaling_update(
    state: &SpinRow,
    monitors: Rc<RefCell<Vec<Monitor>>>,
    monitor_index: usize,
    drawing_area: DrawingArea,
) {
    let scale = state.value();
    let mut monitors = monitors.borrow_mut();
    let monitor = monitors.get_mut(monitor_index).unwrap();
    let original_monitor = monitor.clone();
    let direction = scale > monitor.scale;

    // value is the same as before, no need to do antyhing
    // with the exception of resolution changes -> change to newly appropriate resolution
    if (monitor.scale * 100.0).round() / 100.0 == scale
        && !monitor.drag_information.resolution_changed
    {
        return;
    }
    monitor.drag_information.resolution_changed = false;

    // multiply scale to move at smaller increments
    let mut search_scale = (scale * 120.0).round();
    let mut found = false;

    if is_nonfunctional_scale(monitor.size.0, monitor.size.1, scale) {
        // search the traveled distance for a possible match
        search_nearest_scale(6, &mut search_scale, monitor, direction, &mut found, true);
        // search additional distance if no match has been found
        if !found {
            search_nearest_scale(
                100,
                &mut search_scale,
                monitor,
                direction,
                &mut found,
                false,
            );
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
    rearrange_monitors(original_monitor, monitors);
    drawing_area.queue_draw();
    state
        .activate_action(
            "monitor.reset_monitor_buttons",
            Some(&glib::Variant::from(true)),
        )
        .expect("Could not activate reset action");
}

// fractional scaling can only be done when the scale divides the resolution to a whole
// number.
// Example: 1080 / 1.5 -> 720. E.g. the factor 1.5 will also resolve to a whole number.
fn is_nonfunctional_scale(width: i32, height: i32, scale: f64) -> bool {
    width as f64 % scale != 0.0 && height as f64 % scale != 0.0 && scale != 1.0
}

pub fn search_nearest_scale(
    amount: usize,
    search_scale: &mut f64,
    monitor: &Monitor,
    direction: bool,
    found: &mut bool,
    reverse: bool,
) {
    // reverse x for the second run
    let reverse_scale = if reverse { -1.0 } else { 1.0 };
    for x in 0..amount {
        // increment here does not equal to increment of 1, but 1/120 of an increment
        // specified at: https://wayland.app/protocols/fractional-scale-v1
        let scale_move = if direction {
            (*search_scale + (x as f64) * reverse_scale) / 120.0
        } else {
            (*search_scale - (x as f64) * reverse_scale) / 120.0
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
