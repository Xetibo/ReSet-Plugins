use std::process::Command;

use adw::{ActionRow, gdk, NavigationPage, NavigationView, PreferencesGroup};
use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gdk4::ContentProvider;
use glib::{clone, Variant};
use gtk::{Align, Box, Button, EventController, Label, ListBox, ListBoxRow, Orientation, SearchEntry, WidgetPaintable};
use gtk::{DragSource, prelude::*};
use re_set_lib::utils::plugin::{PluginCapabilities, PluginImplementation, PluginTestFunc, SidebarInfo};
use serde_yaml::Value;

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
pub extern "C" fn frontend_data() -> (SidebarInfo, Vec<Box>) {
    let info = SidebarInfo {
        name: "Keyboard",
        icon_name: "input-keyboard-symbolic",
        parent: None,
    };
    let all_keyboard_layouts = get_keyboard_list();

    let main = Box::builder().orientation(Orientation::Vertical).build();
    main.append(&create_title());

    let nav_view = NavigationView::new();
    main.append(&nav_view);
    
    create_keyboard_main_page(&all_keyboard_layouts, &nav_view);
    create_add_keyboard_page(&all_keyboard_layouts, &nav_view);

    (info, vec![main])
}

fn create_add_keyboard_page(all_keyboard_layouts: &Value, nav_view: &NavigationView) {
    let add_keyboard_page_box = Box::new(Orientation::Vertical, 0);
    let add_keyboard_page = NavigationPage::builder()
        .tag("add_keyboard")
        .child(&add_keyboard_page_box)
        .build();
    nav_view.add(&add_keyboard_page);

    let search_box = Box::new(Orientation::Horizontal, 5);
    add_keyboard_page_box.append(&search_box);

    let search = SearchEntry::builder()
        .placeholder_text("Language or Country")
        .hexpand(true)
        .build();
    search_box.append(&search);

    let add_layout_button = Button::builder()
        .label("Add")
        .sensitive(false)
        .build();
    search_box.append(&add_layout_button);

    let list = ListBox::new();
    add_keyboard_page_box.append(&list);

    list.connect_row_selected(clone!(@weak add_layout_button => move |_, _| {
        add_layout_button.set_sensitive(true);
    }));

    nav_view.connect_popped(clone!(@weak add_layout_button, @weak list => move |_, _| {
        list.unselect_all();
        add_layout_button.set_sensitive(false);
    }));

    add_layout_button.connect_clicked(clone!(@weak nav_view => move |_| {
        nav_view.pop();
        // todo somehow add new layout to saved keyboard layouts
    }));

    for keyboard_layout in all_keyboard_layouts.get("layouts").unwrap().as_sequence().unwrap().iter() {
        let desc = keyboard_layout.get("description").unwrap().as_str().unwrap();
        let layout_row = ListBoxRow::builder()
            .height_request(40)
            .build();
        let layout_row_label = Label::builder()
            .label(desc)
            .halign(Align::Start)
            .build();
        layout_row.set_child(Some(&layout_row_label));
        list.append(&layout_row);
    }

    search.connect_search_changed(clone!(@weak list => move |search_entry| {
        let search_text = search_entry.text();
        if search_text.trim().is_empty() {
            list.unset_filter_func();
            return;
        }

        list.set_filter_func(move |row| {
            let label = row.child().unwrap();
            let label = label.downcast_ref::<Label>().unwrap();
            label.label().to_lowercase().contains(search_text.to_lowercase().as_str())
        });
    }));
}

fn create_keyboard_main_page(all_keyboard_layouts: &Value, nav_view: &NavigationView) {
    let front_page_box = &Box::new(Orientation::Vertical, 0);
    let front_page = NavigationPage::builder()
        .tag("main")
        .child(front_page_box)
        .build();
    nav_view.add(&front_page);

    let keyboard_list = PreferencesGroup::builder()
        .title("Keyboard Layouts")
        .description("Includes keyboard layouts and input methods")
        .build();
    front_page_box.append(&keyboard_list);

    let add_layout_button = Button::builder()
        .icon_name("value-increase-symbolic")
        .valign(Align::Start)
        .build();
    keyboard_list.set_header_suffix(Some(&add_layout_button));
    add_layout_button.set_action_name(Some("navigation.push"));
    add_layout_button.set_action_target_value(Some(&Variant::from("add_keyboard")));
    
    let mut i = 0;
    // todo somehow find where keyboard layouts are saved
    for keyboard_layout in all_keyboard_layouts.get("layouts").unwrap().as_sequence().unwrap().iter() {
        if i > 5 { break; }
        i += 1;

        let desc = keyboard_layout.get("description").unwrap().as_str().unwrap();
        let layout_row = ActionRow::builder().title(desc).build();

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
                keyboard_list.remove(&selected_row);
                listbox.insert(&selected_row, index);
                update_input();
                return true;
            }

            false
        }));

        layout_row.add_controller(EventController::from(source));
        layout_row.add_controller(EventController::from(target));
    }
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

fn get_keyboard_list() -> Value {
    let command_output = Command::new("xkbcli")
        .arg("list")
        .output()
        .expect("failed to execute xkbcli list");

    let output_string = String::from_utf8(command_output.stdout).expect("not utf8");
    let keyboard_layouts: Value = serde_yaml::from_str(&*output_string).unwrap();
    keyboard_layouts
}