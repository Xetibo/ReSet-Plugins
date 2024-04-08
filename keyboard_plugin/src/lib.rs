use adw::{ActionRow, gdk};
use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gdk4::ContentProvider;
use glib::clone;
use glib::prelude::StaticType;
use gtk::{Label, Orientation, WidgetPaintable};
use gtk::prelude::{BoxExt, ButtonExt, WidgetExt};
use re_set_lib::utils::plugin::{PluginCapabilities, PluginImplementation, PluginTestFunc, SidebarInfo};

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
    main.append(&create_title());

    let keyboard_list = adw::PreferencesGroup::new();
    main.append(&keyboard_list);

    // todo fetch keyboard layouts from somewhere ヽ(。_°)ノ

    let layouts = vec![
        ActionRow::builder().title("test").build(),
        ActionRow::builder().title("test2").build(),
        ActionRow::builder().title("test3").build(),
    ];

    for layout in layouts {
        keyboard_list.add(&layout);
        let source = gtk::DragSource::builder()
            .actions(gdk::DragAction::MOVE)
            .build();


        source.connect_prepare(clone!(@weak layout => @default-return None, move |value, x , y| {
            dbg!("source Prepare {:?} and {:?} at ({}, {})", layout.title(), value, x, y);
            let value1 = glib::Value::from(layout);
            Some(ContentProvider::for_value(&value1))
        }));

        source.connect_drag_begin(clone!(@weak layout => move |value, x| {
            dbg!("source Drag Begin {:?} at ({}, {})", value, x);
            layout.add_css_class("selectedLanguage");

            let paintable = WidgetPaintable::new(Some(&layout));
            value.set_icon(Some(&paintable), 0, 0);
        }));

        source.connect_drag_end(clone!(@weak layout => move |value, x, y| {
            dbg!("source Drag end {:?} at ({}, {})", value, x, y);
            layout.remove_css_class("selectedLanguage");
        }));

        let controller = gtk::EventController::from(source);
        layout.add_controller(controller);
    }

    let target = gtk::DropTarget::builder()
        .actions(gdk::DragAction::MOVE)
        .formats(&gdk::ContentFormats::for_type(ActionRow::static_type()))
        .build();

    target.connect_drop(clone!(@weak keyboard_list => @default-return false, move |tar, value, x , y| {
        dbg!("target Dropped {:?} and {:?} at ({}, {})", tar, value, x, y);
        // todo drop not working (╯°□°)╯︵ ┻━┻
        let test = value.get::<ActionRow>();
        keyboard_list.add(&test.unwrap());
        true
    }));

    let controller = gtk::EventController::from(target);
    keyboard_list.add_controller(controller);

    (info, vec![main])
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_tests() -> Vec<PluginTestFunc> {
    println!("frontend tests called");
    vec![]
}

pub struct LabelWrapper {
    label: Label,
}

fn create_title() -> Label {
    Label::builder()
        .label("Keyboard")
        .css_classes(vec!["resetSettingLabel"])
        .halign(gtk::Align::Start)
        .margin_start(5)
        .margin_bottom(10)
        .build()
}

unsafe impl Send for LabelWrapper {}

unsafe impl Sync for LabelWrapper {}