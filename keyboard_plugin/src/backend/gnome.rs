use regex::Regex;
use crate::keyboard_layout::KeyboardLayout;

pub fn get_saved_layouts_gnome(all_keyboards: Vec<KeyboardLayout>) -> Vec<KeyboardLayout> {
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
    todo!()
}