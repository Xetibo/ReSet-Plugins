use std::{cell::RefCell, rc::Rc};

use gtk::{
    gdk::prelude::SurfaceExt,
    prelude::{GestureDragExt, NativeExt},
    DrawingArea, GestureDrag,
};
#[allow(deprecated)]
use gtk::{
    prelude::{BoxExt, StyleContextExt, WidgetExt},
    prelude::{DrawingAreaExtManual, GdkCairoContextExt},
    Orientation,
};
use re_set_lib::utils::plugin::SidebarInfo;

use crate::utils::{get_monitor_data, Monitor};

#[no_mangle]
pub extern "C" fn frontend_startup() {
    println!("frontend startup called");
}

#[no_mangle]
pub extern "C" fn frontend_shutdown() {
    println!("frontend shutdown called");
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_data() -> (SidebarInfo, Vec<gtk::Box>) {
    println!("frontend data called");
    let info = SidebarInfo {
        name: "Monitors",
        icon_name: "preferences-desktop-display-symbolic",
        parent: None,
    };
    // box for the settings
    gtk::init().unwrap();
    let main_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .vexpand(true)
        .build();
    // NOTE: intentional use of deprecated logic as there is no currently available alternative
    // Gnome also uses the same functionality to get the same color for drawing the monitors
    #[allow(deprecated)]
    let context = main_box.style_context();
    #[allow(deprecated)]
    let color = context.lookup_color("headerbar_border_color").unwrap();
    #[allow(deprecated)]
    let border_color = context.lookup_color("accent_color").unwrap();

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
    let drawing_ref_end = drawing_area.clone();

    let monitor_data = Rc::new(RefCell::new(get_monitor_data()));
    let start_ref = monitor_data.clone();
    let update_ref = monitor_data.clone();

    drawing_callback(&drawing_area, border_color, color, monitor_data.clone());
    let gesture = GestureDrag::builder().build();

    // TODO: do something with drag
    gesture.connect_drag_begin(move |_drag, x, y| {
        for monitor in start_ref.borrow_mut().iter_mut() {
            let x = x as i32;
            let y = y as i32;
            if monitor.is_coordinate_within(x, y) {
                monitor.drag_information.drag_active = true;
                break;
            }
        }
        // drawing_ref.queue_draw();

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
        let mut snap_right_left = 0;
        let mut snap_bottom_bottom = 0;
        let mut snap_top_top = 0;
        let mut snap_left_right = 0;
        let mut snap_top_bottom = 0;
        let mut snap_bottom_top = 0;
        let mut snap_right_right = 0;
        let mut snap_left_left = 0;
        // top to top, right to left, bottom to bottom, left to right
        // top to bottom, right to right, bottom to top, left to left
        let mut use_snap = (false, false, false, false, false, false, false, false);
        let mut iter = 0;
        for (i, monitor) in monitor_data.borrow_mut().iter_mut().enumerate() {
            if monitor.drag_information.drag_active {
                monitor.drag_information.drag_active = false;
                endpoint_bottom = monitor.offset.1 + monitor.drag_information.drag_y;
                endpoint_left = monitor.offset.0 + monitor.drag_information.drag_x;
                endpoint_right = endpoint_left + monitor.drag_information.width;
                endpoint_top = endpoint_bottom + monitor.drag_information.height;
                iter = i;
                break;
            }
        }
        for (i, monitor) in monitor_data.borrow_mut().iter_mut().enumerate() {
            if i == iter {
                continue;
            }
            // right to left and left to right
            if endpoint_right.abs_diff(monitor.offset.0) < 100 {
                snap_right_left = monitor.offset.0;
                use_snap.1 = true;
            } else if endpoint_left.abs_diff(monitor.offset.0 + monitor.drag_information.width)
                < 100
            {
                snap_left_right = monitor.offset.0 + monitor.drag_information.width;
                use_snap.3 = true;
            }

            // top to top and bottom to bottom
            if endpoint_bottom.abs_diff(monitor.offset.1) < 100 {
                snap_bottom_bottom = monitor.offset.1;
                use_snap.2 = true;
            } else if endpoint_top.abs_diff(monitor.offset.1 + monitor.drag_information.height)
                < 100
            {
                snap_top_top = monitor.offset.1 - monitor.drag_information.height;
                use_snap.0 = true;
            }

            // right to right and left to left
            if endpoint_right.abs_diff(monitor.offset.0 + monitor.drag_information.width) < 100 {
                snap_right_right = monitor.offset.0 + monitor.drag_information.width;
                use_snap.4 = true;
            } else if endpoint_left.abs_diff(monitor.offset.0) < 100 {
                snap_left_left = monitor.offset.0;
                use_snap.5 = true;
            }

            // top to bottom and bottom to top
            if endpoint_top.abs_diff(monitor.offset.1) < 100 {
                snap_top_bottom = monitor.offset.1;
                use_snap.6 = true;
            } else if endpoint_bottom.abs_diff(monitor.offset.1 + monitor.drag_information.height)
                < 100
            {
                snap_bottom_top = monitor.offset.1 + monitor.drag_information.height;
                use_snap.7 = true;
            }
        }
        let mut monitor = monitor_data.borrow_mut();
        let monitor = monitor.get_mut(iter).unwrap();
        if use_snap.1 {
            println!("snap right to left");
            monitor.offset.0 = snap_right_left - monitor.drag_information.width;
        } else if use_snap.3 {
            println!("snap left to right");
            monitor.offset.0 = snap_left_right;
        } else if use_snap.4 {
            println!("snap right to right");
            monitor.offset.0 = snap_right_right + monitor.drag_information.height;
        } else if use_snap.5 {
            println!("snap left to left");
            monitor.offset.0 = snap_left_left;
        } else {
            monitor.offset.0 += monitor.drag_information.drag_x;
        }
        if use_snap.2 {
            println!("snap bottom to bottom");
            monitor.offset.1 = snap_bottom_bottom;
        } else if use_snap.0 {
            println!("snap top to top");
            monitor.offset.1 = snap_top_top + monitor.drag_information.height;
        } else if use_snap.6 {
            println!("snap top to bottom");
            monitor.offset.1 = snap_top_bottom + monitor.drag_information.height;
        } else if use_snap.7 {
            println!("snap bottom to top");
            monitor.offset.1 = snap_bottom_top;
        } else {
            monitor.offset.1 += monitor.drag_information.drag_y;
        }
        monitor.drag_information.drag_x = 0;
        monitor.drag_information.drag_y = 0;

        drawing_ref_end.queue_draw();
    });

    drawing_area.add_controller(gesture);

    main_box.append(&*drawing_area);

    drawing_area.queue_draw();

    let boxes = vec![main_box];

    (info, boxes)
}

fn drawing_callback(
    area: &DrawingArea,
    border_color: gtk::gdk::RGBA,
    color: gtk::gdk::RGBA,
    monitor_data: Rc<RefCell<Vec<Monitor>>>,
) {
    area.set_draw_func(move |area, context, _, _| {
        // get height of window and width of drawingwidget
        let native = area.native().unwrap();
        let surface = native.surface().unwrap();
        let max_height = surface.height() / 10 * 4;
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
                min_monitor_height = current_min_width;
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

            println!(
                "{} {}",
                monitor.drag_information.drag_x, monitor.drag_information.drag_y
            );

            monitor.drag_information.width = width;
            monitor.drag_information.height = height;
            monitor.drag_information.factor = factor;
            monitor.drag_information.border_offset_x = width_offset;
            monitor.drag_information.border_offset_y = height_offset;

            let offset_x = width_offset + offset_x / factor;
            let offset_y = height_offset + offset_y / factor;
            let height = height / factor;
            let width = width / factor;

            context.set_source_color(&border_color);

            // borders
            // top
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, width + 5, 5);
            context.add_rectangle(&rec);

            // right
            let rec = gtk::gdk::Rectangle::new(offset_x + width, offset_y, 5, height + 5);
            context.add_rectangle(&rec);

            // bottom
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y + height, width + 5, 5);
            context.add_rectangle(&rec);

            // left
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, 5, height + 5);
            context.add_rectangle(&rec);

            context.fill().expect("Could not fill context");

            // monitor
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, width, height);
            context.set_source_color(&color);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
        }
    });
}
