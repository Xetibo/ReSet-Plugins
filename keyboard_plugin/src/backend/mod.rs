use std::ffi::CStr;
use std::process::Command;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use dbus_crossroads::IfaceBuilder;
use re_set_lib::utils::plugin_setup::CrossWrapper;
use xkbregistry::{rxkb_context_new, RXKB_CONTEXT_NO_FLAGS, rxkb_context_parse_default_ruleset, rxkb_context_unref, rxkb_layout_first, rxkb_layout_get_description, rxkb_layout_get_name, rxkb_layout_get_variant, rxkb_layout_next};
use crate::{get_keyboard_list_frontend};
use crate::keyboard_layout::KeyboardLayout;
use crate::r#const::INTERFACE;
use crate::utils::parse_setting;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn dbus_interface(cross: Arc<RwLock<CrossWrapper>>) {
    let mut cross = cross.write().unwrap();
    let interface = setup_dbus_interface(&mut cross);
    let layouts = get_keyboard_list_backend();
    cross.insert::<Vec<KeyboardLayout>>("Keyboard", &[interface], layouts)
}

pub fn setup_dbus_interface(
    cross: &mut RwLockWriteGuard<CrossWrapper>,
) -> dbus_crossroads::IfaceToken<Vec<KeyboardLayout>> {
    cross.register::<Vec<KeyboardLayout>>(
        INTERFACE,
        |c: &mut IfaceBuilder<Vec<KeyboardLayout>>| {
            c.method(
                "GetKeyboardLayouts",
                (),
                ("layouts", ),
                move |_, d: &mut Vec<KeyboardLayout>, ()| {
                    Ok((d.clone(), ))
                },
            );
        },
    )
}

pub fn get_saved_layouts() -> Vec<KeyboardLayout> {
    let all_keyboards = get_keyboard_list_frontend();

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
        if let Some(asdf) =layouts.first() {
            let option = asdf.clone().clone();
            kb.push(option);

        }
    }
    kb.clone()
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