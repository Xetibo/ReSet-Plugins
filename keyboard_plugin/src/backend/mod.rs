use std::ffi::CStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use dbus_crossroads::IfaceBuilder;
use re_set_lib::utils::config::CONFIG;
use re_set_lib::utils::plugin_setup::CrossWrapper;
use xkbregistry::{rxkb_context_new, RXKB_CONTEXT_NO_FLAGS, rxkb_context_parse_default_ruleset, rxkb_context_unref, rxkb_layout_first, rxkb_layout_get_description, rxkb_layout_get_name, rxkb_layout_get_variant, rxkb_layout_next};

use crate::keyboard_layout::KeyboardLayout;
use crate::r#const::INTERFACE;
use crate::utils::{get_default_path, get_environment, parse_setting};

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn dbus_interface(cross: Arc<RwLock<CrossWrapper>>) {
    let mut cross = cross.write().unwrap();
    let interface = setup_dbus_interface(&mut cross);
    cross.insert("Keyboard", &[interface], ());
}

pub fn setup_dbus_interface<T: Send + Sync>(
    cross: &mut RwLockWriteGuard<CrossWrapper>,
) -> dbus_crossroads::IfaceToken<T> {
    cross.register::<T>(
        INTERFACE,
        |c: &mut IfaceBuilder<T>| {
            c.method_with_cr_async(
                "GetKeyboardLayouts",
                (),
                ("layouts", ),
                move |mut ctx, _, ()| async move {
                    ctx.reply(Ok((get_keyboard_list_backend(), )))
                },
            );
            c.method_with_cr_async(
                "GetSavedLayouts",
                (),
                ("layouts", ),
                move |mut ctx, _, ()| async move {
                    ctx.reply(Ok((get_saved_layouts(), )))
                },
            );
            c.method_with_cr_async(
                "SaveLayoutOrder",
                ("layouts", ),
                (),
                move |mut ctx, _, (layouts, ): (Vec<KeyboardLayout>, )| async move {
                    write_to_config(layouts);
                    ctx.reply(Ok(()))
                },
            );
            c.method_with_cr_async(
                "GetMaxActiveKeyboards",
                (),
                ("max", ),
                move |mut ctx, _, ()| async move {
                    ctx.reply(Ok((get_max_active_keyboards(), )))
                },
            );
        },
    )
}

pub fn get_saved_layouts() -> Vec<KeyboardLayout> {
    let all_keyboards = get_keyboard_list_backend();

    let kb_layout = Command::new("hyprctl")
        .arg("getoption")
        .arg("input:kb_layout")
        .output()
        .expect("Failed to get saved layouts");
    let kb_layout = parse_setting(kb_layout);

    let kb_variant = Command::new("hyprctl")
        .arg("getoption")
        .arg("input:kb_variant")
        .output()
        .expect("Failed to get saved variants");
    let mut kb_variant = parse_setting(kb_variant);
    kb_variant.resize(kb_layout.len(), String::new());

    let mut kb = vec![];
    for (layout, variant) in kb_layout.into_iter().zip(kb_variant.into_iter()) {
        let layouts: Vec<&KeyboardLayout> = all_keyboards.iter()
            .filter(|x| x.name == layout.trim())
            .filter(|x| x.variant.as_ref().unwrap_or(&String::new()) == &variant.trim())
            .collect();
        if let Some(asdf) = layouts.first() {
            let option = (*asdf).clone();
            kb.push(option);
        }
    }
    kb
}

fn get_keyboard_list_backend() -> Vec<KeyboardLayout> {
    let mut layouts = vec![];
    unsafe {
        let context = rxkb_context_new(RXKB_CONTEXT_NO_FLAGS);
        rxkb_context_parse_default_ruleset(context);

        let mut layout = rxkb_layout_first(context);
        while !layout.is_null() {
            let description = rxkb_layout_get_description(layout);
            let name = rxkb_layout_get_name(layout);
            let variant = rxkb_layout_get_variant(layout);

            layouts.push(KeyboardLayout {
                description: CStr::from_ptr(description).to_str().unwrap().to_string(),
                name: CStr::from_ptr(name).to_str().unwrap().to_string(),
                variant: if variant.is_null() {
                    None
                } else {
                    Some(CStr::from_ptr(variant).to_str().unwrap().to_string())
                },
            });
            layout = rxkb_layout_next(layout);
        }
        rxkb_context_unref(context);
    }
    layouts
}

fn write_to_config(layouts: Vec<KeyboardLayout>) {
    let path;
    if let Some(test) = CONFIG.get("Keyboard").unwrap().get("path") {
        path = test.as_str().unwrap().to_string();
    } else {
        path = get_default_path();
    }

    let mut input_config = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .open(PathBuf::from(path))
        .expect("Failed to open file");

    let mut layout_string = String::new();
    let mut variant_string = String::new();
    for x in layouts.iter() {
        layout_string += &x.name;
        layout_string += ", ";
        if let Some(var) = &x.variant {
            variant_string += &var;
        }
        variant_string += ", ";
    };

    layout_string = layout_string.trim_end_matches(", ").to_string();
    variant_string = variant_string.trim_end_matches(", ").to_string();

    let string = format!("input {{\n    kb_layout={}\n    kb_variant={}\n}}", layout_string, variant_string);

    input_config.set_len(0).expect("Failed to truncate file");
    input_config.write_all(string.as_bytes()).expect("Failed to write to file");
    input_config.sync_all().expect("Failed to sync file");
}

fn get_max_active_keyboards() -> u32 {
    match get_environment().as_str() {
        "Hyprland" => { 4 }
        _ => { 4 }
    }
}