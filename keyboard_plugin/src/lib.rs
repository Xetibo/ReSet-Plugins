use adw::{ActionRow, gdk};
use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gdk4::ContentProvider;
use glib::clone;
use gtk::{Align, EventController, Label, ListBox, Orientation, WidgetPaintable};
use gtk::{DragSource, prelude::*};
use re_set_lib::utils::plugin::{PluginCapabilities, PluginImplementation, PluginTestFunc, SidebarInfo};

pub const BASE: &str = "org.Xetibo.ReSet.Daemon";
pub const DBUS_PATH: &str = "/org/Xebito/ReSet/Plugins/Keyboard";
pub const INTERFACE: &str = "org.Xetibo.ReSet.Keyboard";

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    println!("frontend capabilities called");
    PluginCapabilities::new(vec!["Keyboard"], PluginImplementation::Frontend)
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

    let keyboard_list = adw::PreferencesGroup::builder()
        .title("Keyboard Layouts")
        .description("Includes keyboard layouts and input methods")
        .build();
    main.append(&keyboard_list);

    let add_layout_button = gtk::Button::builder()
        .icon_name("value-increase-symbolic")
        .valign(Align::Start)
        .build();
    keyboard_list.set_header_suffix(Some(&add_layout_button));

    // todo fetch keyboard layouts from somewhere ヽ(。_°)ノ
    let layouts = vec![
        ActionRow::builder().title("test").build(),
        ActionRow::builder().title("test2").build(),
        ActionRow::builder().title("test3").build(),
    ];

    for layout in layouts {
        keyboard_list.add(&layout);
        let source = DragSource::builder()
            .actions(gdk::DragAction::MOVE)
            .build();

        source.connect_prepare(clone!(@weak layout => @default-return None, move |_, _, _| {
            let value = glib::Value::from(layout);
            Some(ContentProvider::for_value(&value))
        }));

        source.connect_drag_begin(clone!(@weak layout, @weak keyboard_list => move |value, _| {
            layout.add_css_class("selectedLanguage");

            let paintable = WidgetPaintable::new(Some(&layout));
            value.set_icon(Some(&paintable), 0, 0);
        }));

        source.connect_drag_end(clone!(@weak layout => move |_, _, _| {
            layout.remove_css_class("selectedLanguage");
        }));

        let target = gtk::DropTarget::builder()
            .actions(gdk::DragAction::MOVE)
            .formats(&gdk::ContentFormats::for_type(ActionRow::static_type()))
            .build();

        target.connect_drop(clone!(@weak keyboard_list => @default-return false, move |target, value, _ , _| {
            let selected_row = value.get::<ActionRow>().unwrap();
            let droptarget_row = target.widget();
            let droptarget_row = droptarget_row.downcast_ref::<ActionRow>().unwrap();
            
            let listbox = droptarget_row.parent().unwrap();
            let listbox = listbox.downcast_ref::<ListBox>().unwrap();
            
            if droptarget_row.title() != selected_row.title() {
                let index = droptarget_row.index();
                keyboard_list.remove(&selected_row);
                listbox.insert(&selected_row, index);
                update_input();
                return true;
            }

            false
        }));

        layout.add_controller(EventController::from(source));
        layout.add_controller(EventController::from(target));
    }

    (info, vec![main])
}

fn update_input() {
    dbg!("apply input order");
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_tests() -> Vec<PluginTestFunc> {
    println!("frontend tests called");
    vec![]
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
