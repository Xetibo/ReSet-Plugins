use std::{cell::RefCell, rc::Rc};

use glib::clone;
use gtk::{
    gio::{ActionEntry, SimpleActionGroup},
    prelude::FrameExt,
    GestureClick,
};
#[allow(deprecated)]
use gtk::{
    prelude::{
        ActionMapExtManual, BoxExt, ButtonExt, GestureDragExt, StaticVariantType, StyleContextExt,
        WidgetExt,
    },
    GestureDrag, Orientation,
};
use re_set_lib::utils::{gtk::utils::create_title, plugin::SidebarInfo};

use crate::utils::get_monitor_data;

use self::handlers::{
    apply_monitor_clicked, drawing_callback, get_monitor_settings_group, monitor_drag_end,
    monitor_drag_start, monitor_drag_update, reset_monitor_clicked, save_monitor_clicked,
};

pub mod general;
pub mod gnome;
pub mod handlers;

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

    let main_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .vexpand(true)
        .build();
    main_box.append(&create_title("Monitors"));
    let main_box_ref = main_box.clone();

    let apply_row = gtk::Box::new(Orientation::Horizontal, 5);

    let apply = gtk::Button::builder()
        .label("Apply")
        .hexpand_set(false)
        .halign(gtk::Align::End)
        .sensitive(false)
        .build();
    apply_row.append(&apply);

    let save = gtk::Button::builder()
        .label("Save")
        .hexpand_set(false)
        .halign(gtk::Align::End)
        .sensitive(false)
        .build();
    apply_row.append(&save);

    let reset = gtk::Button::builder()
        .label("Reset")
        .hexpand_set(false)
        .halign(gtk::Align::End)
        .sensitive(false)
        .build();
    apply_row.append(&reset);

    main_box.append(&apply_row);

    let settings_box = gtk::Box::new(Orientation::Vertical, 5);
    let settings_box_ref = settings_box.clone();
    let settings_box_ref_apply = settings_box.clone();
    let settings_box_ref_save = settings_box.clone();
    let settings_box_ref_reset = settings_box.clone();
    let settings_box_ref_action = settings_box.clone();

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
    #[allow(deprecated)]
    let clicked_color = context.lookup_color("blue_4").unwrap();

    let drawing_frame = gtk::Frame::builder()
        .margin_top(10)
        .margin_bottom(10)
        .build();

    // NOTE: ensure the size is known before!
    // Otherwise the height or width inside the set_draw_func is 0!
    // E.g. nothing is drawn
    let drawing_area = gtk::DrawingArea::builder()
        .height_request(300)
        .hexpand(true)
        .vexpand(true)
        .build();
    let drawing_ref = drawing_area.clone();
    let drawing_ref_apply = drawing_area.clone();
    let drawing_ref_save = drawing_area.clone();
    let drawing_ref_reset = drawing_area.clone();
    let drawing_ref_end = drawing_area.clone();
    let drawing_ref_action = drawing_area.clone();

    let data = get_monitor_data();
    dbg!(&data);
    let monitor_data = Rc::new(RefCell::new(data.clone()));
    // clone the data for a fallback -> wrong or unusable settings applied
    // return to previous working conditions
    let fall_back_monitor_data = Rc::new(RefCell::new(data));
    let fallback_save_ref = fall_back_monitor_data.clone();
    let fallback_apply_ref = fall_back_monitor_data.clone();
    let fallback_action_ref = fall_back_monitor_data.clone();
    let start_ref = monitor_data.clone();
    let clicked_ref = monitor_data.clone();
    let update_ref = monitor_data.clone();
    let save_ref = monitor_data.clone();

    let apply_ref = monitor_data.clone();
    let apply_action_ref = monitor_data.clone();
    apply.connect_clicked(move |_| {
        apply_monitor_clicked(
            apply_ref.clone(),
            fallback_apply_ref.clone(),
            &settings_box_ref_apply,
            &drawing_ref_apply,
            false,
        );
    });

    let reset_ref = monitor_data.clone();
    reset.connect_clicked(move |button| {
        reset_monitor_clicked(
            reset_ref.clone(),
            &settings_box_ref_reset,
            &drawing_ref_reset,
            button,
        );
    });

    save.connect_clicked(move |_| {
        save_monitor_clicked(
            save_ref.clone(),
            fallback_save_ref.clone(),
            &settings_box_ref_save,
            &drawing_ref_save,
            false,
        );
    });

    {
        let mut monitors_borrow = monitor_data.borrow_mut();
        let monitor = monitors_borrow.get_mut(0);
        if monitor.is_some() {
            monitor.unwrap().drag_information.clicked = true;
        }
    }

    settings_box.append(&get_monitor_settings_group(monitor_data.clone(), 0));

    drawing_callback(
        &drawing_area,
        border_color,
        color,
        dragging_color,
        clicked_color,
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
        monitor_drag_start(x, y, start_ref.clone(), &settings_box_ref);
    });
    gesture.connect_drag_update(move |_drag, x, y| {
        monitor_drag_update(x, y, update_ref.clone(), &drawing_ref);
    });
    gesture.connect_drag_end(move |_drag, _x, _y| {
        monitor_drag_end(monitor_data.clone(), &drawing_ref_end, &main_box_ref);
    });

    drawing_area.add_controller(gesture);
    drawing_area.add_controller(clicked);

    drawing_frame.set_child(Some(&drawing_area));
    let action_group = SimpleActionGroup::new();
    let reset_monitor_buttons = ActionEntry::builder("reset_monitor_buttons")
        .parameter_type(Some(&bool::static_variant_type()))
        .activate(
            clone!(@weak reset, @weak apply, @weak save => move |_, _, description| {
                let enable = description.unwrap().get::<bool>().unwrap();
                apply.set_sensitive(enable);
                save.set_sensitive(enable);
                reset.set_sensitive(enable);
            }),
        )
        .build();
    action_group.add_action_entries([reset_monitor_buttons]);

    let revert_monitors = ActionEntry::builder("revert_monitors")
        .parameter_type(Some(glib::VariantTy::TUPLE))
        .activate(move |_, _, description| {
            let (reverse, persistent) = description.unwrap().get::<(bool, bool)>().unwrap();
            if persistent {
                save_monitor_clicked(
                    apply_action_ref.clone(),
                    fallback_action_ref.clone(),
                    &settings_box_ref_action,
                    &drawing_ref_action,
                    reverse,
                );
            } else {
                apply_monitor_clicked(
                    apply_action_ref.clone(),
                    fallback_action_ref.clone(),
                    &settings_box_ref_action,
                    &drawing_ref_action,
                    reverse,
                );
            }
        })
        .build();
    action_group.add_action_entries([revert_monitors]);
    main_box.insert_action_group("monitor", Some(&action_group));
    main_box.append(&drawing_frame);
    main_box.append(&settings_box);

    drawing_area.queue_draw();

    let boxes = vec![main_box];

    (info, boxes)
}
