use adw::{NavigationPage, NavigationView};
use glib::{clone, Variant};
use gtk::{Align, Button, Label, ListBox, ListBoxRow, Orientation, SearchEntry};
use gtk::prelude::*;

use crate::frontend::get_keyboard_list_frontend;

pub fn create_add_keyboard_page(nav_view: &NavigationView) {
    let add_keyboard_page_box = gtk::Box::new(Orientation::Vertical, 0);
    let add_keyboard_page = NavigationPage::builder()
        .tag("add_keyboard")
        .child(&add_keyboard_page_box)
        .build();
    nav_view.add(&add_keyboard_page);

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
    add_keyboard_page_box.append(&list);
    add_keyboard_list_to_view(&list);

    list.connect_row_selected(clone!(@weak add_layout_button => move |_, _| {
        add_layout_button.set_sensitive(true);
    }));
    
    add_layout_button.connect_clicked(clone!(@weak nav_view, @weak list => move |button| {
        let selected_row = list.selected_row().unwrap();
        let description = selected_row.first_child().unwrap().downcast_ref::<Label>().unwrap().text().to_string();
        let res = button.activate_action("keyboard.addlayout", Some(&Variant::from(description)));
        nav_view.pop();
    }));
    
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

    nav_view.connect_popped(clone!(@weak add_layout_button, @weak list => move |_, _| {
        list.unselect_all();
        add_layout_button.set_sensitive(false);
    }));
}

fn add_keyboard_list_to_view(list: &ListBox) {
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
}
