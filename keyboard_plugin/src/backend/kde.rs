use dbus::blocking::Connection;
use dbus::channel::Sender;
use dbus::Message;
use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::keyboard_layout::KeyboardLayout;
use crate::utils::parse_setting;
use std::process::Command;

pub fn get_saved_layouts_kde(all_keyboards: &[KeyboardLayout]) -> Vec<KeyboardLayout> {
    let output = Command::new("kreadconfig6")
        .arg("--file")
        .arg("kxkbrc")
        .arg("--group")
        .arg("Layout")
        .arg("--key")
        .arg("LayoutList")
        .output()
        .expect("Failed to get saved layouts");
    let kb_layout = parse_setting(output);

    let output = Command::new("kreadconfig6")
        .arg("--file")
        .arg("kxkbrc")
        .arg("--group")
        .arg("Layout")
        .arg("--key")
        .arg("VariantList")
        .output()
        .expect("Failed to get saved layouts");
    let kb_variant = parse_setting(output);

    let mut kb = vec![];
    for (layout, variant) in kb_layout.into_iter().zip(kb_variant.into_iter()) {
        let layouts: Vec<&KeyboardLayout> = all_keyboards
            .iter()
            .filter(|x| x.name == layout.trim())
            .filter(|x| x.variant.as_ref().unwrap_or(&String::new()) == variant.trim())
            .collect();
        if let Some(asdf) = layouts.first() {
            let option = (*asdf).clone();
            kb.push(option);
        }
    }
    kb
}

pub fn write_to_config_kde(layouts: &[KeyboardLayout]) {
    let mut layout_string = String::new();
    let mut variant_string = String::new();
    for x in layouts.iter() {
        layout_string += &x.name;
        layout_string += ", ";
        if let Some(var) = &x.variant {
            variant_string += var;
        }
        variant_string += ", ";
    }

    Command::new("kwriteconfig6")
        .arg("--file")
        .arg("kxkbrc")
        .arg("--group")
        .arg("Layout")
        .arg("--key")
        .arg("LayoutList")
        .arg(layout_string)
        .output()
        .expect("Could not save layouts");
    Command::new("kwriteconfig6")
        .arg("--file")
        .arg("kxkbrc")
        .arg("--group")
        .arg("Layout")
        .arg("--key")
        .arg("VariantList")
        .arg(variant_string)
        .output()
        .expect("Could not save variants");
    let conn = Connection::new_session().unwrap();
    let res =
        conn.send(Message::new_signal("/Layouts", "org.kde.keyboard", "reloadConfig").unwrap());
    if res.is_err() {
        ERROR!("Could not apply config", ErrorLevel::PartialBreakage);
    }
}
