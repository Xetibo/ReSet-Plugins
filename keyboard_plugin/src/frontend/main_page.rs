use std::cell::RefCell;
use std::rc::Rc;

use adw::{ActionRow, NavigationPage, NavigationView, PreferencesGroup};
use adw::gio::{ActionEntry, SimpleActionGroup};
use adw::prelude::PreferencesGroupExt;
use glib::{clone, Variant, VariantTy};
use gtk::{Align, Button, Orientation};
use gtk::prelude::*;

use crate::backend::get_saved_layouts;
use crate::frontend::{add_listener, get_keyboard_list_frontend, update_input};

pub fn create_keyboard_main_page(nav_view: &NavigationView) {
    let mut user_layouts = Rc::new(RefCell::new(get_saved_layouts()));
    let all_keyboard_layouts = get_keyboard_list_frontend();

    let front_page_box = &gtk::Box::new(Orientation::Vertical, 0);
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

    let change_order_entry = ActionEntry::builder("changeorder")
        .parameter_type(Some(&VariantTy::TUPLE))
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

    let action_group = SimpleActionGroup::new();
    let add_layout_entry = ActionEntry::builder("addlayout")
        .parameter_type(Some(&String::static_variant_type()))
        .activate(clone!(@weak keyboard_list, @strong user_layouts => move |_, _, description| {
            let layout_description = description.unwrap().str().unwrap();
            let layout_row = ActionRow::builder().title(layout_description).build();
            {
                let mut user_layout_borrow= user_layouts.borrow_mut();
                let layout = all_keyboard_layouts.iter()
                    .find(|x| x.description == layout_description)
                    .unwrap();
                
                user_layout_borrow.push(layout.clone());
                keyboard_list.add(&layout_row);
                add_listener(&keyboard_list, layout_row);
            }
            update_input(&user_layouts);
        }))
        .build();
    action_group.add_action_entries([add_layout_entry, change_order_entry]);
    nav_view.insert_action_group("keyboard", Some(&action_group));

    let add_layout_button = Button::builder()
        .icon_name("value-increase-symbolic")
        .valign(Align::Start)
        .build();
    keyboard_list.set_header_suffix(Some(&add_layout_button));
    add_layout_button.set_action_name(Some("navigation.push"));
    add_layout_button.set_action_target_value(Some(&Variant::from("add_keyboard")));

    for layout in user_layouts.borrow().iter() {
        let layout_row = ActionRow::builder().title(layout.description.clone()).build();
        add_listener(&keyboard_list, layout_row);
    }
}
