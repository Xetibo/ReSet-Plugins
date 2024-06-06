use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use re_set_lib::utils::config::CONFIG;
use re_set_lib::ERROR;
#[cfg(debug_assertions)]
use re_set_lib::{utils::macros::ErrorLevel, write_log_to_file};

use crate::keyboard_layout::KeyboardLayout;
use crate::utils::{get_default_path, parse_setting};

pub fn get_saved_layouts_hyprland(all_keyboards: &[KeyboardLayout]) -> Vec<KeyboardLayout> {
    let kb_layout = Command::new("hyprctl")
        .arg("getoption")
        .arg("input:kb_layout")
        .output();
    if kb_layout.is_err() {
        ERROR!(
            "Failed to fetch keyboard layouts",
            ErrorLevel::PartialBreakage
        );
        return vec![];
    }
    let kb_layout = parse_setting(kb_layout.unwrap());

    let kb_variant = Command::new("hyprctl")
        .arg("getoption")
        .arg("input:kb_variant")
        .output();
    if kb_variant.is_err() {
        ERROR!(
            "Failed to fetch keyboard variants",
            ErrorLevel::PartialBreakage
        );
        return vec![];
    }
    let mut kb_variant = parse_setting(kb_variant.unwrap());
    kb_variant.resize(kb_layout.len(), String::new());

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

pub fn write_to_config_hyprland(layouts: Vec<KeyboardLayout>) {
    let path;
    #[allow(clippy::borrow_interior_mutable_const)]
    if let Some(test) = CONFIG.get("Keyboard").unwrap().get("path") {
        path = test.as_str().unwrap().to_string();
    } else {
        path = get_default_path();
    }

    let mut input_config = OpenOptions::new()
        .write(true)
        .read(true)
        .open(PathBuf::from(path))
        .expect("Failed to open file");

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

    layout_string = layout_string.trim_end_matches(", ").to_string();
    variant_string = variant_string.trim_end_matches(", ").to_string();

    let string = format!(
        "input {{\n    kb_layout={}\n    kb_variant={}\n}}",
        layout_string, variant_string
    );

    input_config.set_len(0).expect("Failed to truncate file");
    input_config
        .write_all(string.as_bytes())
        .expect("Failed to write to file");
    input_config.sync_all().expect("Failed to sync file");
}
