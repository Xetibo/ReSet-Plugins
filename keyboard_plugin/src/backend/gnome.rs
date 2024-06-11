use glib::{Variant, VariantTy};
use gtk::prelude::SettingsExtManual;
use std::process::Command;

use crate::keyboard_layout::KeyboardLayout;

pub fn get_saved_layouts_gnome(all_keyboards: &[KeyboardLayout]) -> Vec<KeyboardLayout> {
    let mut kb = vec![];

    let mut result = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.input-sources", "sources"])
        .output();
    if result.is_err() {
        return kb;
    }
    let mut output = result.unwrap();
    let mut layout_variant = String::from_utf8(output.stdout).unwrap();

    if layout_variant.contains("@a(ss)") {
        result = Command::new("flatpak-spawn")
            .args([
                "--host",
                "gsettings",
                "get",
                "org.gnome.desktop.input-sources",
                "sources",
            ])
            .output();
        if result.is_err() {
            return kb;
        }
        output = result.unwrap();
        layout_variant = String::from_utf8(output.stdout).unwrap();
    }

    let layout_variant = Variant::parse(Some(VariantTy::new("a(ss)").unwrap()), &layout_variant);
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
        let mut option = x.variant.unwrap_or(String::new());
        if !option.is_empty() {
            option = "+".to_string() + &option;
        }
        all_layouts.push(("xkb", x.name.clone() + &option));
    }

    let variant = Variant::from(all_layouts);
    let input_sources = gtk::gio::Settings::new("org.gnome.desktop.input-sources");
    input_sources
        .set("sources", variant)
        .expect("failed to write layouts");
}
