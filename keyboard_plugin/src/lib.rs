use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::time::Duration;

use adw::{ActionRow, gdk, NavigationPage, NavigationView, PreferencesGroup};
use adw::gio::{ActionEntry, SimpleActionGroup};
use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use dbus::{arg, Error, Path, Signature};
use dbus::arg::{Append, Arg, ArgType, Get};
use dbus::blocking::Connection;
use dbus_crossroads::IfaceBuilder;
use gdk4::ContentProvider;
use glib::{clone, Variant};
use gtk::{Align, Box, Button, EventController, Label, ListBox, ListBoxRow, Orientation, SearchEntry, WidgetPaintable};
use gtk::{DragSource, prelude::*};
use gtk::FileChooserAction::Open;
use re_set_lib::utils::config::CONFIG;
use re_set_lib::utils::plugin::{PluginCapabilities, PluginImplementation, PluginTestFunc, SidebarInfo};
use re_set_lib::utils::plugin_setup::CrossWrapper;
use xkbregistry::{rxkb_context_new, RXKB_CONTEXT_NO_FLAGS, rxkb_context_parse_default_ruleset, rxkb_context_unref, rxkb_layout_first, rxkb_layout_get_description, rxkb_layout_get_name, rxkb_layout_get_variant, rxkb_layout_next};

pub const BASE: &str = "org.Xetibo.ReSet.Daemon";
pub const DBUS_PATH: &str = "/org/Xebito/ReSet/Plugins/Keyboard";
pub const INTERFACE: &str = "org.Xetibo.ReSet.Keyboard";
pub const DEFAULT_PATH: &str = "~/.config/reset/keyboard.conf";

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    println!("frontend capabilities called");
    PluginCapabilities::new(vec!["Keyboard"], true, PluginImplementation::Both)
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

    let main = Box::builder().orientation(Orientation::Vertical).build();
    main.append(&create_title());

    let nav_view = NavigationView::new();
    main.append(&nav_view);

    create_keyboard_main_page(&nav_view);
    create_add_keyboard_page(&nav_view);

    (info, vec![main])
}

fn create_add_keyboard_page(nav_view: &NavigationView) {
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

    add_layout_button.connect_clicked(clone!(@weak nav_view, @weak list => move |button| {
        let selected_row = list.selected_row().unwrap();
        let description = selected_row.first_child().unwrap().downcast_ref::<Label>().unwrap().text().to_string();
        let res = button.activate_action("keyboard.addlayout", Some(&Variant::from(description)));

        update_input();
        nav_view.pop();
    }));

    for keyboard_layout in get_keyboard_list_frontend() {
        let layout_row = ListBoxRow::builder()
            .height_request(40)
            .build();
        let layout_row_label = Label::builder()
            .label(keyboard_layout.description)
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

fn create_keyboard_main_page(nav_view: &NavigationView) {
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

    let action_group = SimpleActionGroup::new();
    let entry = ActionEntry::builder("addlayout")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(clone!(@weak keyboard_list => move |_, _, description| {
            let layout_description = description.unwrap().str().unwrap();
            let layout_row = ActionRow::builder().title(layout_description).build();
            add_listener(&keyboard_list, layout_row);
        }))
        .build();
    action_group.add_action_entries([entry]);
    nav_view.insert_action_group("keyboard", Some(&action_group));

    let add_layout_button = Button::builder()
        .icon_name("value-increase-symbolic")
        .valign(Align::Start)
        .build();
    keyboard_list.set_header_suffix(Some(&add_layout_button));
    add_layout_button.set_action_name(Some("navigation.push"));
    add_layout_button.set_action_target_value(Some(&Variant::from("add_keyboard")));
    
    let mut i = 0;
    // todo somehow find where keyboard layouts are saved
    for keyboard_layout in get_keyboard_list_frontend() {
        if i > 5 { break; }
        i += 1;

        let layout_row = ActionRow::builder().title(keyboard_layout.description).build();
        add_listener(&keyboard_list, layout_row);
    }
}

fn add_listener(keyboard_list: &PreferencesGroup, layout_row: ActionRow) {
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

fn get_keyboard_list_frontend() -> Vec<KeyboardLayouts> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(Vec<KeyboardLayouts>, ), Error> = proxy.method_call(INTERFACE, "GetKeyboardLayouts", ());
    if res.is_err() {
        return Vec::new();
    }
    res.unwrap().0
}

fn update_input() {
    dbg!("apply input order");
    let path;
    if let Some(test) = CONFIG.get("Keyboard").unwrap().get("path") {
        path = test.as_str().unwrap().to_string();
    } else {
        path = getDefaultPath();
    }
    
    let mut input_config = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(PathBuf::from(path))
        .expect("Failed to open file");

    let string = format!("input {{\n    kb_layout={}\n    kb_variant={}\n}}", "ch, us", "");

    let result = input_config.write_all(string.as_bytes());
    let result1 = input_config.sync_all();
    result.expect("Failed to write to file");
    result1.expect("Failed to sync file");
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn name() -> String {
    String::from("Keyboard")
}

#[no_mangle]
pub extern "C" fn backend_startup() {
    // todo use default path if no path set
    println!("startup called");
}

#[no_mangle]
pub extern "C" fn backend_shutdown() {
    println!("shutdown called");
}

fn get_keyboard_list_backend() -> Vec<KeyboardLayouts> {
    let mut layouts = vec![];
    unsafe {
        let context = rxkb_context_new(RXKB_CONTEXT_NO_FLAGS);
        rxkb_context_parse_default_ruleset(context);

        let mut layout = rxkb_layout_first(context);
        while !layout.is_null() {
            let description = rxkb_layout_get_description(layout);
            let name = rxkb_layout_get_name(layout);
            let variant = rxkb_layout_get_variant(layout);

            layouts.push(KeyboardLayouts {
                description: CStr::from_ptr(description).to_str().unwrap().to_string(),
                name: CStr::from_ptr(name).to_str().unwrap().to_string(),
                variant: if variant.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(variant).to_str().unwrap().to_string())
                },
            });
            layout = rxkb_layout_next(layout);
        }
        rxkb_context_unref(context);
    }
    layouts
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn dbus_interface(cross: Arc<RwLock<CrossWrapper>>) {
    let mut cross = cross.write().unwrap();
    let interface = setup_dbus_interface(&mut cross);
    let layouts = get_keyboard_list_backend();
    cross.insert::<Vec<KeyboardLayouts>>("Keyboard", &[interface], layouts)
}

pub fn setup_dbus_interface(
    cross: &mut RwLockWriteGuard<CrossWrapper>,
) -> dbus_crossroads::IfaceToken<Vec<KeyboardLayouts>> {
    cross.register::<Vec<KeyboardLayouts>>(
        INTERFACE,
        |c: &mut IfaceBuilder<Vec<KeyboardLayouts>>| {
            c.method(
                "GetKeyboardLayouts",
                (),
                ("layouts", ),
                move |_, d: &mut Vec<KeyboardLayouts>, ()| {
                    Ok((d.clone(), ))
                },
            );
        },
    )
}

pub fn getSavedLayouts() {
    // TODO parse
    let kb_layout = Command::new("hyprctl getoption input:kb_layout")
        .output()
        .expect("Failed to get saved layouts");
    let kb_variant = Command::new("hyprctl getoption input:kb_variant")
        .output()
        .expect("Failed to get saved variants");
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct KeyboardLayouts {
    pub description: String,
    pub name: String,
    pub variant: Option<String>,
}

fn create_title() -> Label {
    Label::builder()
        .label("Keyboard")
        .css_classes(vec!["resetSettingLabel"])
        .halign(Align::Start)
        .margin_start(5)
        .margin_bottom(10)
        .build()
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_tests() -> Vec<PluginTestFunc> {
    println!("frontend tests called");
    vec![]
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn backend_tests() -> Vec<PluginTestFunc> {
    println!("tests called");
    Vec::new()
}

impl Append for KeyboardLayouts {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        let variant;
        if self.variant.is_none() {
            variant = String::from("None");
        } else {
            variant = self.variant.clone().unwrap();
        }

        iter.append_struct(|i| {
            i.append(self.description.clone());
            i.append(self.name.clone());
            i.append(variant);
        });
    }
}

impl<'a> Get<'a> for KeyboardLayouts {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (description, name, variant, ) = <(String, String, String, )>::get(i)?;
        Some(Self {
            description,
            name,
            variant: if variant == "None" {
                None
            } else {
                Some(variant)
            },
        })
    }
}

impl Arg for KeyboardLayouts {
    const ARG_TYPE: ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(sss)\0") }
    }
}

fn getDefaultPath() -> String {
    let dirs = directories_next::ProjectDirs::from("org", "Xetibo", "ReSet")
        .unwrap();
    let buf = dirs.config_dir()
        .join("keyboard.conf");
    let path = buf
        .to_str()
        .unwrap();
    String::from(path)
}
