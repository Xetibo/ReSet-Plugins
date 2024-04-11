use std::{cell::RefCell, rc::Rc};

use glib::translate::Uninitialized;
use gtk::{
    builders::{EventControllerMotionBuilder, GestureDragBuilder},
    cairo::{ffi::cairo_matrix_transform_point, Matrix},
    ffi::{GtkEventController, GtkEventControllerMotion},
    gdk::{prelude::SurfaceExt, Rectangle},
    prelude::GestureExt,
    prelude::{GestureDragExt, NativeExt},
    DrawingArea, EventController, EventControllerMotion, GestureDrag,
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

    let rectangles = drawing_callback(&drawing_area, border_color, color, monitor_data.clone());
    let rectangles_drag_start = rectangles.clone();
    let rectangles_drag_update = rectangles.clone();
    let rectangles_drag_end = rectangles.clone();
    let gesture = GestureDrag::builder().build();

    // TODO: do something with drag
    gesture.connect_drag_begin(move |drag, x, y| {
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

    gesture.connect_drag_update(move |drag, x, y| {
        for monitor in update_ref.borrow_mut().iter_mut() {
            let x = x as i32;
            let y = y as i32;
            if monitor.drag_information.drag_active {
                monitor.drag_information.drag_x = x;
                monitor.drag_information.drag_y = -y;
                break;
            }
        }
        drawing_ref.queue_draw();
    });

    gesture.connect_drag_end(move |drag, x, y| {
        let mut endpoint_left: i32 = 0;
        let mut endpoint_bottom: i32 = 0;
        let mut endpoint_right: i32 = 0;
        let mut endpoint_top: i32 = 0;
        let mut snap_right = 0;
        let mut snap_bottom = 0;
        let mut snap_top = 0;
        let mut snap_left = 0;
        let mut iter = 0;
        for (i, monitor) in monitor_data.borrow_mut().iter_mut().enumerate() {
            if monitor.drag_information.drag_active {
                monitor.drag_information.drag_active = false;
                endpoint_right = monitor.drag_information.scaled_offset_x
                    + monitor.drag_information.scaled_width;
                endpoint_top = monitor.drag_information.scaled_offset_y;
                endpoint_bottom = monitor.drag_information.scaled_offset_y
                    - monitor.drag_information.scaled_height;
                endpoint_left = monitor.drag_information.scaled_offset_y;
                iter = i;
                break;
            }
        }
        for (i, monitor) in monitor_data.borrow_mut().iter_mut().enumerate() {
            if i == iter {
                continue;
            }
            if endpoint_right.abs_diff(monitor.drag_information.scaled_offset_x) < 10 {
                snap_right = monitor.drag_information.scaled_offset_x;
            }
            if endpoint_left.abs_diff(
                monitor.drag_information.scaled_offset_x + monitor.drag_information.scaled_width,
            ) < 10
            {
                snap_left = monitor.drag_information.scaled_offset_x
                    + monitor.drag_information.scaled_width;
            }
            println!(
                "difference is {} ",
                endpoint_bottom.abs_diff(monitor.drag_information.scaled_offset_y)
            );
            if endpoint_bottom.abs_diff(monitor.drag_information.scaled_offset_y) < 10 {
                snap_bottom = monitor.drag_information.scaled_offset_y;
            }
            if endpoint_top.abs_diff(
                monitor.drag_information.scaled_offset_y - monitor.drag_information.scaled_height,
            ) < 10
            {
                snap_top = monitor.drag_information.scaled_offset_y
                    - monitor.drag_information.scaled_height;
            }
        }
        let mut monitor = monitor_data.borrow_mut();
        let monitor = monitor.get_mut(iter).unwrap();
        if snap_right != 0 {
            println!("snapped horizontally to right at {}", snap_right);
            monitor.offset.0 = snap_right - monitor.drag_information.scaled_width;
        }
        if snap_bottom != 0 {
            println!("snapped vertically to bottom at {}", snap_bottom);
            monitor.offset.1 = snap_bottom - monitor.drag_information.scaled_height;
        }
        if snap_top != 0 {
            println!("snapped vertically to top at {}", snap_top);
            monitor.offset.1 = snap_bottom + monitor.drag_information.scaled_height;
        }
        if snap_left != 0 {
            println!("snapped horizontally to left at {}", snap_left);
            monitor.offset.0 = snap_right + monitor.drag_information.scaled_width;
        }

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
) -> Rc<RefCell<Vec<Rectangle>>> {
    let rectangles = Rc::new(RefCell::new(Vec::new()));
    let rectangle_ref = rectangles.clone();
    area.set_draw_func(move |area, context, _, _| {
        // get height of window and width of drawingwidget
        let native = area.native().unwrap();
        let surface = native.surface().unwrap();
        let max_height = surface.height() / 3;
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
            let current_max_height = monitor.offset.1.abs() + height;
            let current_max_width = monitor.offset.0.abs() + width;
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
        let width_factor = max_monitor_width / max_width + 2;
        let height_factor = max_monitor_height / max_height + 2;
        let factor = if width_factor > height_factor {
            width_factor
        } else {
            height_factor
        };
        let width_offset = (max_width - (max_monitor_width / factor)) / 2;
        let height_offset = (max_height - (max_monitor_height / max_height)) / 2;

        let mut rectangled = rectangle_ref.borrow_mut();
        for monitor in monitor_data.borrow_mut().iter_mut() {
            // handle transform which could invert height and width
            let (width, height) = monitor.handle_transform();
            let height = height / factor;
            let width = width / factor;
            let offset_x = min_monitor_width
                + width_offset
                + monitor.drag_information.drag_x
                + monitor.offset.0 / factor;
            let offset_y = min_monitor_height + max_height
                - height_offset * 2
                - monitor.drag_information.drag_y
                - (monitor.offset.1 / factor);
            monitor.drag_information.scaled_offset_x = offset_x;
            monitor.drag_information.scaled_offset_y = offset_y;
            monitor.drag_information.scaled_width = width;
            monitor.drag_information.scaled_height = height;

            context.set_source_color(&border_color);

            // borders
            // top
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, width + 5, 5);
            context.add_rectangle(&rec);
            rectangled.push(rec);

            // right
            let rec = gtk::gdk::Rectangle::new(offset_x + width, offset_y, 5, height + 5);
            context.add_rectangle(&rec);
            rectangled.push(rec);

            // bottom
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y + height, width + 5, 5);
            context.add_rectangle(&rec);
            rectangled.push(rec);

            // left
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, 5, height + 5);
            context.add_rectangle(&rec);
            rectangled.push(rec);

            context.fill().expect("Could not fill context");

            // monitor
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y, width, height);
            context.set_source_color(&color);
            context.add_rectangle(&rec);
            rectangled.push(rec);
            context.fill().expect("Could not fill context");
        }
    });
    rectangles
}
