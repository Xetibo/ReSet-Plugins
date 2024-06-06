use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use adw::gio::{ActionEntry, SimpleActionGroup};
use adw::prelude::{ActionRowExt, PreferencesGroupExt};
use adw::{ActionRow, NavigationPage, NavigationView, PreferencesGroup};
use dbus::blocking::Connection;
use dbus::Error;
use glib::{clone, Variant, VariantTy};
use gtk::prelude::*;
use gtk::{Align, Button, Image, ListBox, Orientation, Popover};

use crate::frontend::{add_listener, get_keyboard_list_frontend, update_input};
use crate::keyboard_layout::KeyboardLayout;
use crate::r#const::{BASE, DBUS_PATH, INTERFACE};
use crate::utils::get_max_active_keyboards;

pub fn create_keyboard_main_page(nav_view: &NavigationView) {
    let max_keyboards = get_max_active_keyboards();
    let user_layouts = Rc::new(RefCell::new(get_saved_layouts_frontend()));

    let all_keyboard_layouts = get_keyboard_list_frontend();

    let front_page_box = &gtk::Box::new(Orientation::Vertical, 0);
    let front_page = NavigationPage::builder()
        .tag("main")
        .child(front_page_box)
        .title("Keyboard Settings")
        .build();
    nav_view.add(&front_page);

    let keyboard_list = PreferencesGroup::builder()
        .title("Keyboard Layouts")
        .description("Only the first four layouts will be active")
        .build();
    front_page_box.append(&keyboard_list);

    let action_group = SimpleActionGroup::new();
    let change_order_entry = ActionEntry::builder("changeorder")
        .parameter_type(Some(VariantTy::TUPLE))
        .activate(clone!(@strong user_layouts => move |_, _, description| {
            let (from_index, to_index) = description.unwrap().get::<(i32, i32)>().unwrap();
            {
                let mut user_layout_borrow= user_layouts.borrow_mut();
                let layout = user_layout_borrow[from_index as usize].clone();

                user_layout_borrow.remove(from_index as usize);
                user_layout_borrow.insert(to_index as usize, layout);
            }
            update_input(&user_layouts);
        }))
        .build();

    let add_layout_entry = ActionEntry::builder("addlayout")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(
            clone!(@weak keyboard_list, @strong user_layouts => move |_, _, description| {
                let layout_description = description.unwrap().str().unwrap();

                let layout_row = create_action_row(layout_description.to_string());
                {
                    let mut user_layout_borrow= user_layouts.borrow_mut();
                    let layout = all_keyboard_layouts.iter()
                        .find(|x| x.description == layout_description)
                        .unwrap();

                    user_layout_borrow.push(layout.clone());
                    keyboard_list.add(&layout_row);

                    if layout_row.index() < max_keyboards as i32 {
                        layout_row.add_css_class("activeLanguage");
                    }

                    add_listener(&keyboard_list, layout_row);
                }

                update_input(&user_layouts);
            }),
        )
        .build();

    let remove_layout = ActionEntry::builder("removeLayout")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(
            clone!(@weak keyboard_list, @strong user_layouts => move |_, _, description| {
                let layout = description.unwrap().str().unwrap();
                user_layouts.borrow_mut().retain(|x| x.description != layout);
                update_input(&user_layouts);
            }),
        )
        .build();

    action_group.add_action_entries([add_layout_entry, change_order_entry, remove_layout]);
    nav_view.insert_action_group("keyboard", Some(&action_group));

    let add_layout_button = Button::builder()
        .icon_name("value-increase-symbolic")
        .valign(Align::Start)
        .build();
    keyboard_list.set_header_suffix(Some(&add_layout_button));
    add_layout_button.set_action_name(Some("navigation.push"));
    add_layout_button.set_action_target_value(Some(&Variant::from("add_keyboard")));

    for (index, layout) in user_layouts.borrow().iter().enumerate() {
        let layout_row = create_action_row(layout.description.clone());

        if index < max_keyboards as usize {
            layout_row.add_css_class("activeLanguage");
        }
        add_listener(&keyboard_list, layout_row);
    }
}

fn get_saved_layouts_frontend() -> Vec<KeyboardLayout> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(Vec<KeyboardLayout>,), Error> =
        proxy.method_call(INTERFACE, "GetSavedLayouts", ());
    if res.is_err() {
        return Vec::new();
    }
    res.unwrap().0
}

fn create_action_row(title: String) -> ActionRow {
    let action_row = ActionRow::builder().title(title.clone()).build();

    let drag_icon = Image::from_icon_name("list-drag-handle-symbolic");
    action_row.add_prefix(&drag_icon);

    let menu = Button::builder()
        .icon_name("view-more-symbolic")
        .valign(Align::Center)
        .has_frame(false)
        .build();

    let remove_layout = Button::builder()
        .icon_name("edit-delete-symbolic")
        .label("Remove")
        .has_frame(false)
        .can_focus(false)
        .build();

    let layout_popover = Popover::builder().child(&remove_layout).build();

    let suffix_box = gtk::Box::new(Orientation::Horizontal, 0);
    suffix_box.append(&menu);
    suffix_box.append(&layout_popover);
    action_row.add_suffix(&suffix_box);

    menu.connect_clicked(clone!(@weak layout_popover => move |_| {
        layout_popover.popup();
    }));

    remove_layout.connect_clicked(clone!(@strong title, @weak action_row => move |button| {
        button.activate_action("keyboard.removeLayout", Some(&Variant::from(title.clone())))
            .expect("Could not activate action.");
        let parent = action_row.parent().unwrap();
        let parent = parent.downcast_ref::<ListBox>().unwrap();
        parent.remove(&action_row);
    }));

    action_row
}

