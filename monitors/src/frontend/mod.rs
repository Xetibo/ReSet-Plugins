use std::{
    cell::RefCell, cmp::Ordering, collections::HashSet, f64::consts, rc::Rc, time::Duration,
};

use adw::{
    prelude::{ActionRowExt, ComboRowExt, PreferencesGroupExt, PreferencesRowExt},
    PreferencesGroup,
};
use dbus::{arg::RefArg, blocking::Connection, Error};
use glib::object::CastNone;
#[allow(deprecated)]
use gtk::{
    gdk::prelude::SurfaceExt,
    prelude::{
        BoxExt, ButtonExt, DrawingAreaExtManual, GdkCairoContextExt, GestureDragExt, NativeExt,
        StyleContextExt, WidgetExt,
    },
    DrawingArea, GestureDrag, Orientation, StringList,
};
use gtk::{GestureClick, StringObject};
use re_set_lib::{utils::plugin::SidebarInfo, LOG};

use crate::{
    r#const::{BASE, DBUS_PATH, INTERFACE},
    utils::{
        get_environment, get_monitor_data, Monitor, SnapDirectionHorizontal, SnapDirectionVertical,
    },
};

#[no_mangle]
pub extern "C" fn frontend_startup() {
    adw::init().unwrap();
}

#[no_mangle]
pub extern "C" fn frontend_shutdown() {}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_data() -> (SidebarInfo, Vec<gtk::Box>) {
    let info = SidebarInfo {
        name: "Monitors",
        icon_name: "preferences-desktop-display-symbolic",
        parent: None,
    };
    // box for the settings
    let main_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .vexpand(true)
        .build();

    let apply_row = gtk::Box::new(Orientation::Horizontal, 5);

    let apply = gtk::Button::new();
    apply.set_label("Apply");
    apply.set_hexpand(false);
    apply.set_halign(gtk::Align::End);
    apply_row.append(&apply);

    let reset = gtk::Button::new();
    reset.set_label("Reset");
    reset.set_hexpand(false);
    reset.set_halign(gtk::Align::End);
    apply_row.append(&reset);

    main_box.append(&apply_row);

    // TODO: permantently save
    // if is_config_writable() {
    //     let save_row = gtk::Box::new(Orientation::Vertical, 5);
    //     let save = gtk::Button::new();
    //     save.set_label("Save");
    //     save.set_hexpand(false);
    //     save.set_halign(gtk::Align::End);
    //     save_row.append(&save);
    //     main_box.append(&save_row);
    // }

    let settings_box = gtk::Box::new(Orientation::Vertical, 5);
    let settings_box_ref = settings_box.clone();
    let settings_box_ref_apply = settings_box.clone();
    let settings_box_ref_reset = settings_box.clone();
    // NOTE: intentional use of deprecated logic as there is no currently available alternative
    // Gnome also uses the same functionality to get the same color for drawing the monitors
    #[allow(deprecated)]
    let context = settings_box.style_context();
    #[allow(deprecated)]
    let color = context.lookup_color("headerbar_border_color").unwrap();
    #[allow(deprecated)]
    let border_color = context.lookup_color("accent_color").unwrap();
    #[allow(deprecated)]
    let dragging_color = context.lookup_color("blue_5").unwrap();

    // NOTE: ensure the size is known before!
    // Otherwise the height or width inside the set_draw_func is 0!
    // E.g. nothing is drawn
    let drawing_area = Rc::new(
        gtk::DrawingArea::builder()
            .height_request(300)
            .hexpand(true)
            .vexpand(true)
            .build(),
    );
    let drawing_ref = drawing_area.clone();
    let drawing_ref_apply = drawing_area.clone();
    let drawing_ref_reset = drawing_area.clone();
    let drawing_ref_end = drawing_area.clone();

    let monitor_data = Rc::new(RefCell::new(get_monitor_data()));
    let start_ref = monitor_data.clone();
    let clicked_ref = monitor_data.clone();
    let update_ref = monitor_data.clone();

    let apply_ref = monitor_data.clone();
    apply.connect_clicked(move |_| {
        let conn = Connection::new_session().unwrap();
        let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
        let res: Result<(), Error> =
            proxy.method_call(INTERFACE, "SetMonitors", (apply_ref.borrow().clone(),));
        if res.is_err() {
            println!("error on save");
        }
        if let Some(child) = settings_box_ref_apply.first_child() {
            settings_box_ref_apply.remove(&child);
        }
        let mut index = 0;
        for (i, monitor) in apply_ref.borrow().iter().enumerate() {
            if monitor.drag_information.clicked {
                index = i;
            };
        }
        apply_ref.replace(get_monitor_data());
        settings_box_ref_apply.append(&get_monitor_settings_group(apply_ref.clone(), index));
        drawing_ref_apply.queue_draw();
    });

    let reset_ref = monitor_data.clone();
    reset.connect_clicked(move |_| {
        if let Some(child) = settings_box_ref_reset.first_child() {
            settings_box_ref_reset.remove(&child);
        }
        let mut index = 0;
        for (i, monitor) in reset_ref.borrow_mut().iter_mut().enumerate() {
            monitor.offset.0 = monitor.drag_information.origin_x;
            monitor.offset.1 = monitor.drag_information.origin_y;
            if monitor.drag_information.clicked {
                index = i;
            };
        }
        reset_ref.replace(get_monitor_data());
        settings_box_ref_reset.append(&get_monitor_settings_group(reset_ref.clone(), index));
        drawing_ref_reset.queue_draw();
    });

    settings_box.append(&get_monitor_settings_group(monitor_data.clone(), 0));

    drawing_callback(
        &drawing_area,
        border_color,
        color,
        dragging_color,
        monitor_data.clone(),
    );
    let clicked = GestureClick::builder().build();

    clicked.connect_pressed(move |_, _, x, y| {
        for monitor in clicked_ref.borrow_mut().iter_mut() {
            let x = x as i32;
            let y = y as i32;
            if monitor.is_coordinate_within(x, y) {
                monitor.drag_information.clicked = true;
            } else if monitor.drag_information.clicked {
                monitor.drag_information.clicked = false;
            }
        }
    });

    let gesture = GestureDrag::builder().build();

    gesture.connect_drag_begin(move |_drag, x, y| {
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

        // TODO: get the area, check for overlap with rectangles, if so, drag and drop
    });

    gesture.connect_drag_update(move |_drag, x, y| {
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
    });

    gesture.connect_drag_end(move |_drag, _x, _y| {
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
                monitor.drag_information.drag_active = false;
                endpoint_bottom = monitor.offset.1 + monitor.drag_information.drag_y;
                endpoint_left = monitor.offset.0 + monitor.drag_information.drag_x;
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
            let endpoint_other_top = endpoint_other_bottom + monitor.drag_information.height;

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

            if monitor.get_intersect(
                endpoint_left,
                endpoint_bottom,
                previous_width,
                previous_height,
            ) && (snap_vertical == SnapDirectionVertical::None
                || snap_horizontal == SnapDirectionHorizontal::None)
            {
                intersected = true;
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
                SnapDirectionHorizontal::LeftRight(snap)
                | SnapDirectionHorizontal::LeftLeft(snap) => {
                    monitor.offset.0 = snap;
                }
                SnapDirectionHorizontal::None => {
                    monitor.offset.0 += monitor.drag_information.drag_x
                }
            }
            match snap_vertical {
                SnapDirectionVertical::TopTop(snap) | SnapDirectionVertical::TopBottom(snap) => {
                    monitor.offset.1 = snap - monitor.drag_information.height;
                }
                SnapDirectionVertical::BottomTop(snap)
                | SnapDirectionVertical::BottomBottom(snap) => {
                    monitor.offset.1 = snap;
                }
                SnapDirectionVertical::None => monitor.offset.1 += monitor.drag_information.drag_y,
            }
        }
        monitor.drag_information.drag_x = 0;
        monitor.drag_information.drag_y = 0;

        drawing_ref_end.queue_draw();
    });

    drawing_area.add_controller(gesture);
    drawing_area.add_controller(clicked);

    main_box.append(&*drawing_area);
    main_box.append(&settings_box);

    drawing_area.queue_draw();

    let boxes = vec![main_box];

    (info, boxes)
}

fn get_monitor_settings_group(
    clicked_monitor: Rc<RefCell<Vec<Monitor>>>,
    index: usize,
) -> PreferencesGroup {
    let settings = PreferencesGroup::new();

    let name = adw::ComboRow::new();
    let monitors = clicked_monitor.borrow();
    let monitor = monitors.get(index).unwrap();
    name.set_title(&monitor.name);
    name.set_subtitle(&monitor.make);
    name.set_sensitive(true);
    settings.add(&name);

    dbg!(&monitor.available_modes);

    let vrr = adw::SwitchRow::new();
    vrr.set_title("Variable Refresh-Rate");
    vrr.set_active(monitor.vrr);
    let vrr_ref = clicked_monitor.clone();
    vrr.connect_active_notify(move |state| {
        vrr_ref.borrow_mut().get_mut(index).unwrap().vrr = state.is_active();
    });
    settings.add(&vrr);

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
        _ => println!("Unexpected value for transform"),
    }
    let transform_ref = clicked_monitor.clone();
    transform.connect_selected_item_notify(move |dropdown| {
        let mut monitor = transform_ref.borrow_mut();
        let monitor = monitor.get_mut(index).unwrap();
        match model_list.string(dropdown.selected()).unwrap().as_str() {
            "0" => monitor.transform = 0,
            "90" => monitor.transform = 1,
            "180" => monitor.transform = 2,
            "270" => monitor.transform = 3,
            "0-flipped" => monitor.transform = 4,
            "90-flipped" => monitor.transform = 5,
            "180-flipped" => monitor.transform = 6,
            "270-flipped" => monitor.transform = 7,
            _ => println!("Unexpected value for transform"),
        }
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
        let selected = dropdown.selected_item();
        let selected = selected.and_downcast_ref::<StringObject>().unwrap();
        let selected = selected.string().to_string();
        let (x, y) = selected.split_once('x').unwrap();
        let mut monitor = resolution_ref.borrow_mut();
        let monitor = monitor.get_mut(index).unwrap();
        let refresh_rates = monitor
            .available_modes
            .get(index)
            .unwrap()
            .refresh_rates
            .clone();
        let highest = refresh_rates.first().unwrap();
        monitor.refresh_rate = *highest;
        monitor.size.0 = x.parse().unwrap();
        monitor.size.1 = y.parse().unwrap();

        let refresh_rates: Vec<String> = refresh_rates.iter().map(|x| x.to_string()).collect();
        let refresh_rates: Vec<&str> = refresh_rates.iter().map(|x| x.as_str()).collect();
        let refresh_rate_model = StringList::new(&refresh_rates);
        refresh_rate_combo_ref.set_model(Some(&refresh_rate_model));
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
    dbg!(&refresh_rates);
    let refresh_rate_model = StringList::new(&refresh_rates);
    refresh_rate.set_model(Some(&refresh_rate_model));
    // let mut refresh_rate_set: Vec<u32> = refresh_rate_set.into_iter().collect();
    // refresh_rate_set.sort_unstable_by(|a, b| {
    //     if a < b {
    //         Ordering::Greater
    //     } else {
    //         Ordering::Less
    //     }
    // });
    // dbg!(&refresh_rate_set);
    // let model_list = StringList::new(&[]);
    // let mut index = 0;
    // for (i, rate) in refresh_rate_set.into_iter().enumerate() {
    //     if rate == monitor.refresh_rate {
    //         index = i;
    //     }
    //     model_list.append(&rate.to_string());
    // }
    refresh_rate.set_title("Refresh-Rate");
    refresh_rate.set_selected(index as u32);
    let refresh_rate_ref = clicked_monitor.clone();
    refresh_rate.connect_selected_item_notify(move |dropdown| {
        refresh_rate_ref
            .borrow_mut()
            .get_mut(index)
            .unwrap()
            .refresh_rate = dropdown
            .selected_item()
            .and_downcast_ref::<StringObject>()
            .unwrap()
            .string()
            .to_string()
            .parse()
            .unwrap();
    });
    settings.add(&refresh_rate);

    // let model_list = StringList::new(&["this", "should", "be", "taken", "from", "the", "monitor"]);
    // let primary = adw::ComboRow::new();
    // primary.set_title("Primary Monitor");
    // primary.set_model(Some(&model_list));
    // primary.connect_selected_item_notify(move |_state| {
    //     println!("clicked on primary");
    // });
    // settings.add(&primary);
    settings
}

fn drawing_callback(
    area: &DrawingArea,
    border_color: gtk::gdk::RGBA,
    color: gtk::gdk::RGBA,
    draggin_color: gtk::gdk::RGBA,
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
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, width, height);
            if monitor.drag_information.drag_active {
                context.set_source_color(&draggin_color);
            } else {
                context.set_source_color(&color);
            }
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");

            // borders
            context.set_source_color(&border_color);

            context.set_line_width(5.0);
            // top
            // let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, width + 5, 5);
            let rec = gtk::gdk::Rectangle::new(offset_x + 5, offset_y - 5, width - 5, 5);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
            context.arc(
                offset_x as f64 + 7.5,
                (offset_y + height) as f64 - 2.5,
                5.0,
                consts::FRAC_PI_2,
                consts::PI,
            );
            context.stroke().expect("Could not fill context");

            // right
            // let rec = gtk::gdk::Rectangle::new(offset_x + width, offset_y, 5, height + 5);
            let rec = gtk::gdk::Rectangle::new(offset_x + width, offset_y, 5, height);
            context.add_rectangle(&rec);
            // arcs are radian...
            context.fill().expect("Could not fill context");
            context.arc(
                (offset_x + width) as f64 - 2.5,
                offset_y as f64 + 2.5,
                5.0,
                consts::PI + consts::FRAC_PI_2,
                consts::PI * 2.0,
            );
            context.stroke().expect("Could not fill context");

            // bottom
            // let rec = gtk::gdk::Rectangle::new(offset_x, offset_y + height, width + 5, 5);
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
            // let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, 5, height + 5);
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, 5, height);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
            context.arc(
                offset_x as f64 + 7.5,
                offset_y as f64 + 2.5,
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
