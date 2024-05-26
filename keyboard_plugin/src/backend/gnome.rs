use std::process::Command;

use regex::Regex;

use crate::keyboard_layout::KeyboardLayout;

pub fn get_saved_layouts_gnome(all_keyboards: &Vec<KeyboardLayout>) -> Vec<KeyboardLayout> {
    let mut kb = vec![];
    let result = dconf_rs::get_string("/org/gnome/desktop/input-sources/sources");

    if let Ok(layouts) = result {
        let pattern = Regex::new(r"[a-zA-Z0-9_+-]+").unwrap();
        for layout in pattern.captures_iter(layouts.as_str()) {
            let layout = &layout[0];
            let kb_layout: Vec<&KeyboardLayout>;
            if layout.contains("+") {
                let kb_data: Vec<&str> = layout.split("+").collect();
                kb_layout = all_keyboards.iter()
                    .filter(|x| x.name == kb_data[0].trim())
                    .filter(|x| x.variant.as_ref().unwrap_or(&String::new()) == kb_data[1].trim())
                    .collect();
            } else {
                kb_layout = all_keyboards.iter()
                    .filter(|x| x.name == layout.trim())
                    .filter(|x| x.variant.is_none())
                    .collect();
            }
            if let Some(option) = kb_layout.first() {
                kb.push((*option).clone());
            }
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

    Command::new("dconf")
        .arg("write")
        .arg("/org/gnome/desktop/input-sources/sources")
        .arg(all_layouts)
        .status()
        .expect("failed to execute command");
}