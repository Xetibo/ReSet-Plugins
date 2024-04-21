use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use adw::{ActionRow, gdk, PreferencesGroup};
use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use dbus::arg::Append;
use dbus::blocking::Connection;
use dbus::Error;
use gdk4::ContentProvider;
use glib::{clone, Variant};
use gtk::{Align, EventController, Label, ListBox, WidgetPaintable};
use gtk::{DragSource, prelude::*};
use re_set_lib::utils::config::CONFIG;

use crate::keyboard_layout::KeyboardLayout;
use crate::r#const::{BASE, DBUS_PATH, INTERFACE};
use crate::utils::get_default_path;

pub mod main_page;
pub mod add_layout_page;

pub fn get_keyboard_list_frontend() -> Vec<KeyboardLayout> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(Vec<KeyboardLayout>, ), Error> = proxy.method_call(INTERFACE, "GetKeyboardLayouts", ());
    if res.is_err() {
        return Vec::new();
    }
    res.unwrap().0
}

pub fn add_listener(keyboard_list: &PreferencesGroup, layout_row: ActionRow) {
    keyboard_list.add(&layout_row);
    let source = DragSource::builder()
        .actions(gdk::DragAction::MOVE)
        .build();

    source.connect_prepare(clone!(@weak layout_row => @default-return None, move |_, _, _| {
            let value = glib::Value::from(layout_row);
            Some(ContentProvider::for_value(&value))
        }));

    source.connect_drag_begin(clone!(@weak layout_row, @weak keyboard_list => move |value, _| {
            layout_row.add_css_class("selectedLanguage");

            let paintable = WidgetPaintable::new(Some(&layout_row));
            value.set_icon(Some(&paintable), 0, 0);
        }));

    source.connect_drag_end(clone!(@weak layout_row => move |_, _, _| {
            layout_row.remove_css_class("selectedLanguage");
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
                let from_to = Variant::from((selected_row.index(), index));
                keyboard_list.remove(&selected_row);
                listbox.insert(&selected_row, index);
                let res = keyboard_list.activate_action("keyboard.changeorder", Some(&from_to));
                return true;
            }

            false
        }));
    layout_row.add_controller(EventController::from(source));
    layout_row.add_controller(EventController::from(target));
}

pub fn update_input(user_layouts: &Rc<RefCell<Vec<KeyboardLayout>>>) {
    let path;
    if let Some(test) = CONFIG.get("Keyboard").unwrap().get("path") {
        path = test.as_str().unwrap().to_string();
    } else {
        path = get_default_path();
    }

    let mut input_config = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(PathBuf::from(path))
        .expect("Failed to open file");

    let mut layout_string = String::new();
    let mut variant_string = String::new();
    for x in user_layouts.borrow().iter() {
        layout_string += &x.name;
        layout_string += ", ";
        if let Some(var) = &x.variant {
            variant_string += &var;
        }
        variant_string += ", ";
    };

    layout_string = layout_string.trim_end_matches(", ").to_string();
    variant_string = variant_string.trim_end_matches(", ").to_string();

    let string = format!("input {{\n    kb_layout={}\n    kb_variant={}\n}}", layout_string, variant_string);

    input_config.write_all(string.as_bytes()).expect("Failed to write to file");
    input_config.sync_all().expect("Failed to sync file");
}

pub fn create_title() -> Label {
    Label::builder()
        .label("Keyboard")
        .css_classes(vec!["resetSettingLabel"])
        .halign(Align::Start)
        .margin_start(5)
        .margin_bottom(10)
        .build()
}
