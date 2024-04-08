use adw::{ActionRow, gdk};
use adw::prelude::PreferencesGroupExt;
use dbus::arg::messageitem::MessageItem::Str;
use gdk4::ContentProvider;
use glib::clone;
use glib::gobject_ffi::GValue;
use glib::prelude::StaticType;
use gtk::Orientation;
use gtk::prelude::{BoxExt, ButtonExt, WidgetExt};
use re_set_lib::utils::plugin::{PluginCapabilities, PluginImplementation, PluginTestFunc, SidebarInfo};
use adw::prelude::PreferencesRowExt;

pub const BASE: &str = "org.Xetibo.ReSet.Daemon";
pub const DBUS_PATH: &str = "/org/Xebito/ReSet/Plugins/test";
pub const INTERFACE: &str = "org.Xetibo.ReSet.TestPlugin";

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    println!("frontend capabilities called");
    PluginCapabilities::new(vec!["frontend test"], PluginImplementation::Frontend)
}

#[no_mangle]
pub extern "C" fn frontend_startup() {
    adw::init().expect("Adw failed to initialize");
}

#[no_mangle]
pub extern "C" fn frontend_shutdown() {
    println!("frontend shutdown called");
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_data() -> (SidebarInfo, Vec<gtk::Box>) {
    let info = SidebarInfo {
        name: "Keyboard",
        icon_name: "input-keyboard-symbolic",
        parent: None,
    };

    let main = gtk::Box::builder().orientation(Orientation::Vertical).build();

    let title = gtk::Label::builder()
        .label("Keyboard")
        .css_classes(vec!["resetSettingLabel"])
        .halign(gtk::Align::Start)
        .margin_start(5)
        .margin_bottom(10)
        .build();
    main.append(&title);

    let keyboard_list = adw::PreferencesGroup::new();
    main.append(&keyboard_list);

    let row = ActionRow::builder().title("test").build();
    let row2 = ActionRow::builder().title("test2").build();
    let row3 = ActionRow::builder().title("test3").build();
    keyboard_list.add(&row);
    keyboard_list.add(&row2);
    keyboard_list.add(&row3);


    let source = gtk::DragSource::builder()
        .actions(gdk::DragAction::MOVE)
        .build();

    source.set_icon(Some(&gdk::Paintable::new_empty(100, 100)), 100, 100);

    source.connect_prepare(clone!(@weak row => @default-return None, move |value, x , y| {
        dbg!("source Prepare {:?} and {:?} at ({}, {})", row.title(), value, x, y);
        let value1 = glib::Value::from("asldfks");
        Some(ContentProvider::for_value(&value1))
    }));

    // source.connect_prepare(move |value, x, y| {
    //     dbg!("source Prepare {:?} at ({}, {})", value, x, y);
    //     let value1 = glib::Value::from("asldfks");
    //     Some(ContentProvider::for_value(&value1))
    // });

    source.connect_drag_begin(move |value, x| {
        dbg!("source Drag Begin {:?} at ({}, {})", value, x);
    });

    source.connect_drag_end(move |value, x, y| {
        dbg!("source Drag end {:?} at ({}, {})", value, x, y);
    });

    let controller = gtk::EventController::from(source);
    row.add_controller(controller);

    let target = gtk::DropTarget::builder()
        .actions(gdk::DragAction::MOVE)
        .formats(&gdk::ContentFormats::for_type(String::static_type()))
        .build();

    target.connect_drop(clone!(@weak keyboard_list => @default-return false, move |tar, value, x , y| {
        dbg!("target Dropped {:?} at ({}, {})", value, x, y);
        false
    }));

    let controller = gtk::EventController::from(target);
    keyboard_list.add_controller(controller);
    // main.add_controller(controller);


    let boxes = vec![
        main
    ];

    (info, boxes)
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_tests() -> Vec<PluginTestFunc> {
    println!("frontend tests called");
    vec![]
}

pub struct LabelWrapper {
    label: gtk::Label,
}

unsafe impl Send for LabelWrapper {}

unsafe impl Sync for LabelWrapper {}