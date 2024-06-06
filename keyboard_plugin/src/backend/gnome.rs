use glib::{Variant, VariantType};
use gtk::prelude::{SettingsExt, SettingsExtManual};

use crate::keyboard_layout::KeyboardLayout;

pub fn get_saved_layouts_gnome(all_keyboards: &[KeyboardLayout]) -> Vec<KeyboardLayout> {
    let mut kb = vec![];
    let input_sources = gtk::gio::Settings::new("org.gnome.desktop.input-sources");
    let layout_variant = input_sources.value("sources");
    if layout_variant.type_() != VariantType::new("a(ss)").unwrap() {
        return kb;
    }

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
