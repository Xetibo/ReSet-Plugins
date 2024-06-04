use std::collections::HashMap;

use adw::{ActionRow, NavigationPage, NavigationView, PreferencesGroup};
use adw::prelude::{ActionRowExt, PreferencesGroupExt, PreferencesRowExt};
use glib::{clone, SignalHandlerId, Variant};
use glib::translate::FromGlib;
use gtk::{Button, GestureClick, Image, ListBox, Orientation, ScrolledWindow, SearchEntry};
use gtk::prelude::*;

use crate::frontend::get_keyboard_list_frontend;
use crate::keyboard_layout::KeyboardLayout;

// todo remove language should select fourth language if possible

pub fn create_add_keyboard_page(nav_view: &NavigationView) {
    let add_keyboard_page_box = gtk::Box::new(Orientation::Vertical, 0);
    let add_keyboard_page = NavigationPage::builder()
        .tag("add_keyboard")
        .child(&add_keyboard_page_box)
        .build();
    nav_view.add(&add_keyboard_page);

    let back_group = PreferencesGroup::builder()
        .margin_bottom(10)
        .build();

    let back_button = ActionRow::builder()
        .title("Back")
        .activatable(true)
        .action_name("navigation.pop")
        .build();
    
    back_button.add_suffix(&Image::from_icon_name("go-previous-symbolic"));
    back_group.add(&back_button);
    add_keyboard_page_box.append(&back_group);

    let search_box = gtk::Box::new(Orientation::Horizontal, 5);
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
    let scroll_window = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .min_content_height(500)
        .css_classes(vec!["boxed-list"])
        .build();
    scroll_window.set_child(Some(&list));

    add_keyboard_page_box.append(&scroll_window);
    add_keyboard_list_to_view(&list);

    list.connect_row_selected(clone!(@weak add_layout_button => move |_, _| {
        add_layout_button.set_sensitive(true);
    }));

    add_layout_button.connect_clicked(clone!(@weak nav_view, @weak list => move |button| {
        let selected_row = list.selected_row().unwrap();
        let selected_row = selected_row.downcast_ref::<ActionRow>();
        let description = selected_row.unwrap().title().to_string();
        button.activate_action("keyboard.addlayout", Some(&Variant::from(description)))
            .expect("Failed to activate action.");
        nav_view.pop();
    }));

    search.connect_search_changed(clone!(@weak list => move |search_entry| {
        let search_text = search_entry.text();
        if search_text.trim().is_empty() {
            list.unset_filter_func();
            return;
        }

        list.set_filter_func(move |row| {
            let action_row = row.downcast_ref::<ActionRow>().unwrap();
            action_row.title().to_lowercase().contains(search_text.to_lowercase().as_str())
        });
    }));

    nav_view.connect_popped(clone!(@weak add_layout_button, @weak list => move |_, _| {
        list.unselect_all();
        add_layout_button.set_sensitive(false);
    }));
}

fn add_keyboard_list_to_view(list: &ListBox) {
    list.grab_focus();
    let keyboard_layouts = get_keyboard_list();

    let id = list.connect_row_selected(move |_, option| {
        if let Some(layout_row) = option {
            let action_row = layout_row.downcast_ref::<ActionRow>().unwrap();
            if action_row.title() == "Back" {
                return;
            }
            action_row.emit_activate();
        }
    });

    let back_row = create_layout_row("Back".to_string());
    back_row.add_prefix(&Image::from_icon_name("go-previous-symbolic"));
    let click = GestureClick::builder().build();

    click.connect_pressed(clone!(@weak list => move |_, _, _, _| {
        unsafe {
            let asdf = SignalHandlerId::from_glib(id.as_raw());
            list.disconnect(asdf);
        }
        list.remove_all();
        list.unselect_all();
        add_keyboard_list_to_view(&list);
    }));

    back_row.add_controller(click);
    for (_, description, keyboard_layouts) in keyboard_layouts {
        let layout_row = create_layout_row(description);
        list.append(&layout_row);

        if keyboard_layouts.len() == 1 {
            continue;
        } else {
            layout_row.add_suffix(&Image::from_icon_name("go-previous-symbolic-rtl"));

            layout_row.connect_activate(clone!(@strong keyboard_layouts, @weak list, @strong back_row => move |_| {
                
                for keyboard_layout in keyboard_layouts.clone() {
                    let layout_row = create_layout_row(keyboard_layout.description.clone());
                    list.append(&layout_row);
                }
                
                list.prepend(&back_row);

                // remove all but first
                let mut last_row = list.last_child();
                let mut skip = keyboard_layouts.len();
                
                while last_row != None {
                    if list.first_child() == last_row {
                        let second_row = &last_row.unwrap().next_sibling().unwrap();
                        let second_row = second_row.downcast_ref::<ActionRow>();
                        list.select_row(second_row);
                        break;
                    }
                    if skip > 0 {
                        last_row = last_row.unwrap().prev_sibling();
                        skip -= 1;
                        continue;
                    }
                    let temp = last_row.clone().unwrap().prev_sibling();
                    list.remove(&last_row.unwrap());
                    last_row = temp;
                }
                list.unselect_all();

            }));
        }
    }
}

fn get_keyboard_list() -> Vec<(String, String, Vec<KeyboardLayout>)> {
    let mut collection = HashMap::new();
    let mut collected_layouts = vec![];

    for layout in get_keyboard_list_frontend() {
        let variant = layout.variant.clone();
        let description = layout.description.clone();
        let name = layout.name.clone();
        collection.entry(layout.name.clone())
            .or_insert((String::new(), Vec::new())).1
            .push(layout);
        if variant.is_none() {
            collection.get_mut(&name).unwrap().0 = description;
        }
    }
    for (name, (description, mut layouts)) in collection {
        layouts.sort_by(|a, b| a.description.cmp(&b.description));
        collected_layouts.push((name, description, layouts));
    }
    collected_layouts.sort_by(|a, b| a.1.cmp(&b.1));

    collected_layouts
}

fn create_layout_row(string: String) -> ActionRow {
    ActionRow::builder()
        .height_request(40)
        .title(string)
        .build()
}
