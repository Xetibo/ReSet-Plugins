use gtk::{
    builders::{EventControllerMotionBuilder, GestureDragBuilder},
    ffi::{GtkEventController, GtkEventControllerMotion},
    gdk::prelude::SurfaceExt,
    prelude::GestureDragExt,
    prelude::NativeExt,
    DrawingArea, EventController, EventControllerMotion, GestureDrag,
};
#[allow(deprecated)]
use gtk::{
    prelude::{BoxExt, StyleContextExt, WidgetExt},
    prelude::{DrawingAreaExtManual, GdkCairoContextExt},
    Orientation,
};
use re_set_lib::utils::plugin::SidebarInfo;

use crate::utils::get_monitor_data;

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
    let drawing_area = gtk::DrawingArea::builder()
        .height_request(300)
        .hexpand(true)
        .vexpand(true)
        .build();

    drawing_callback(&drawing_area, border_color, color);
    let gesture = GestureDrag::builder().build();

    // TODO: do something with drag
    gesture.connect_drag_begin(|drag, x, y| {
        println!("start drag at {} {} ", x, y);
        // TODO: get the area, check for overlap with rectangles, if so, drag and drop
    });

    gesture.connect_drag_end(|drag, x, y| {});

    drawing_area.add_controller(gesture);

    main_box.append(&drawing_area);

    drawing_area.queue_draw();

    let boxes = vec![main_box];

    (info, boxes)
}

fn drawing_callback(area: &DrawingArea, border_color: gtk::gdk::RGBA, color: gtk::gdk::RGBA) {
    let monitor_data = get_monitor_data();
    area.set_draw_func(move |area, context, _, _| {
        // get height of window and width of drawinwidget
        let native = area.native().unwrap();
        let surface = native.surface().unwrap();
        let max_height = surface.height() / 3;
        let max_width = area.width();

        area.set_height_request(max_height);
        area.height_request();

        // logic to ensure max width and height with offsets and sized do not overflow drawing area
        let mut max_monitor_width = 0;
        let mut max_monitor_height = 0;
        for monitor in monitor_data.iter() {
            let (width, height) = monitor.handle_transform();
            let current_max_height = monitor.offset.1.abs() + height;
            let current_max_width = monitor.offset.0.abs() + width;
            if current_max_width > max_monitor_width {
                max_monitor_width = current_max_width;
            }
            if current_max_height > max_monitor_height {
                max_monitor_height = current_max_height;
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

        for monitor in monitor_data.iter() {
            // handle transform which could invert height and width
            let (width, height) = monitor.handle_transform();
            let height = height / factor;
            let width = width / factor;
            let offset_x = width_offset + monitor.offset.0 / factor;
            let offset_y = max_height - height_offset * 2 - (monitor.offset.1 / factor);

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
