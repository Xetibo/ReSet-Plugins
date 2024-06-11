use std::process::Command;

use glib::{Variant, VariantTy};
use gtk::prelude::SettingsExtManual;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};
use re_set_lib::ERROR;

use crate::keyboard_layout::KeyboardLayout;

pub fn get_saved_layouts_gnome(all_keyboards: &[KeyboardLayout]) -> Vec<KeyboardLayout> {
    let mut kb = vec![];

    let mut result = Command::new("dconf")
        .args(&["read", "/org/gnome/desktop/input-sources/sources"])
        .output();
    if result.is_err() {
        return kb;
    }
    let mut output = result.unwrap();
    let mut layout_variant = String::from_utf8(output.stdout).unwrap();

    if layout_variant.contains("@a(ss)") {
        result = Command::new("flatpak-spawn")
            .args(&["--host", "dconf", "read", "/org/gnome/desktop/input-sources/sources"])
            .output();
        if result.is_err() {
            return kb;
        }
        output = result.unwrap();
        layout_variant = String::from_utf8(output.stdout).unwrap();
    }

    let layout_variant = Variant::parse(Some(&VariantTy::new("a(ss)").unwrap()), &layout_variant);
    if layout_variant.is_err() {
        return kb;
    }
    let layout_variant = layout_variant.unwrap();
    let layouts = layout_variant.get::<Vec<(String, String)>>().unwrap();
    for layout in layouts {
        let kb_layout: Vec<&KeyboardLayout> = if layout.1.contains("+") {
            let kb_data: Vec<&str> = layout.1.split("+").collect();
            all_keyboards
                .iter()
                .filter(|x| x.name == kb_data[0])
                .filter(|x| x.variant.as_ref().unwrap_or(&String::new()) == kb_data[1].trim())
                .collect()
        } else {
            all_keyboards
                .iter()
                .filter(|x| x.name == layout.1.trim())
                .filter(|x| x.variant.is_none())
                .collect()
        };
        if let Some(option) = kb_layout.first() {
            kb.push((*option).clone());
        }
    }
    kb
}

pub fn write_to_config_gnome(layouts: Vec<KeyboardLayout>) {
    let mut all_layouts = vec![];
    for x in layouts {
        let mut layout_string = format!("('xkb', '{}", x.name.clone());
        if let Some(var) = x.variant {
            layout_string += &format!("+{}", var);
        }
        layout_string += "')";
        all_layouts.push(layout_string);
    }

    let mut all_layouts = all_layouts.join(", ");
    all_layouts.insert_str(0, "[");
    all_layouts.push_str("]");

    let result = Command::new("dconf")
        .args(&["read", "/org/gnome/desktop/input-sources/sources"])
        .output();
    if result.is_err() {
        return;
    }
    let output = result.unwrap();
    let layout_variant = String::from_utf8(output.stdout).unwrap();

    if layout_variant.contains("@a(ss)") {
        let result = Command::new("flatpak-spawn")
            .args(&["--host", "dconf", "write", "/org/gnome/desktop/input-sources/sources", all_layouts.as_str()])
            .output();
        if result.is_err() {
            ERROR!("Failed to write layouts", ErrorLevel::PartialBreakage);
        }
    } else { 
        let result = Command::new("dconf")
            .args(&["write", "/org/gnome/desktop/input-sources/sources", all_layouts.as_str()])
            .output();
        if result.is_err() {
            ERROR!("Failed to write layouts", ErrorLevel::PartialBreakage);
        }
    }
}
