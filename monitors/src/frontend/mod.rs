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
        .orientation(Orientation::Horizontal)
        .build();
    // NOTE: intentional use of deprecated logic as there is no currently available alternative
    #[allow(deprecated)]
    let context = main_box.style_context();
    #[allow(deprecated)]
    let color = context.lookup_color("headerbar_border_color").unwrap();
    #[allow(deprecated)]
    let border_color = context.lookup_color("accent_color").unwrap();

    // somehow we have to call this again?
    let drawing_area = gtk::DrawingArea::new();
    // NOTE: ensure the size is known before!
    // Otherwise the height or width inside the set_draw_func is 0!
    // E.g. nothing is drawn
    drawing_area.set_height_request(300);
    drawing_area.set_width_request(500);
    main_box.append(&drawing_area);
    drawing_area.set_draw_func(move |_, context, _, max_height| {
        let monitor_data = get_monitor_data();
        // area.dr
        for (i, monitor) in monitor_data.iter().enumerate() {
            let height = monitor.size.1 / 10;
            let width = monitor.size.0 / 10;
            let offset_x = monitor.offset.0 / 10 + i as i32 * 5;
            let offset_y = max_height - 5 - height - (monitor.offset.1 / 10);
            println!("{} {} {} {}", height, width, offset_x, offset_y);
            // context.set_source_color(&RGBA::new(1.0, 0.0, 0.0, 1.0));
            context.set_source_color(&border_color);

            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y - 5, 5, height + 10);
            context.add_rectangle(&rec);
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y - 5, width + 10, 5);
            context.add_rectangle(&rec);
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y + height, width + 10, 5);
            context.add_rectangle(&rec);
            let rec = gtk::gdk::Rectangle::new(offset_x + 5 + width, offset_y - 5, 5, height + 10);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");

            let rec = gtk::gdk::Rectangle::new(offset_x + 5, offset_y, width, height);
            context.set_source_color(&color);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
        }
    });
    drawing_area.queue_draw();

    let boxes = vec![main_box];

    (info, boxes)
}
