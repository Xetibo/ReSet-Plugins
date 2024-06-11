use std::ffi::CStr;
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use dbus_crossroads::IfaceBuilder;
use re_set_lib::utils::plugin_setup::CrossWrapper;
use xkbregistry::{
    rxkb_context_new, rxkb_context_parse_default_ruleset, rxkb_context_unref, rxkb_layout_first,
    rxkb_layout_get_description, rxkb_layout_get_name, rxkb_layout_get_variant, rxkb_layout_next,
    RXKB_CONTEXT_NO_FLAGS,
};

use crate::backend::gnome::{get_saved_layouts_gnome, write_to_config_gnome};
use crate::backend::hyprland::{get_saved_layouts_hyprland, write_to_config_hyprland};
use crate::backend::kde::{get_saved_layouts_kde, write_to_config_kde};
use crate::keyboard_layout::KeyboardLayout;
use crate::r#const::{GNOME, HYPRLAND, INTERFACE, KDE};
use crate::utils::get_environment;

mod gnome;
mod hyprland;
mod kde;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn dbus_interface(cross: Arc<RwLock<CrossWrapper>>) {
    let mut cross = cross.write().unwrap();
    let interface = setup_dbus_interface(&mut cross);
    cross.insert("Keyboard", &[interface], ());
}

pub fn get_saved_layouts() -> Vec<KeyboardLayout> {
    let all_keyboards = get_keyboard_list_backend();

    let env = get_environment();
    if env.contains(GNOME) {
        return get_saved_layouts_gnome(&all_keyboards);
    }

    match env.as_str() {
        HYPRLAND => get_saved_layouts_hyprland(&all_keyboards),
        GNOME => get_saved_layouts_gnome(&all_keyboards),
        KDE => get_saved_layouts_kde(&all_keyboards),
        _ => {
            let kb = vec![];
            kb
        }
    }
}

fn write_to_config(layouts: Vec<KeyboardLayout>) {
    let env = get_environment();
    if env.contains(GNOME) {
        return write_to_config_gnome(&layouts);
    }

    match env.as_str() {
        HYPRLAND => {
            write_to_config_hyprland(&layouts);
        }
        GNOME => {
            write_to_config_gnome(&layouts);
        }
        KDE => {
            write_to_config_kde(&layouts);
        }
        _ => {}
    }
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

fn get_max_active_keyboards() -> u32 {
    match get_environment().as_str() {
        HYPRLAND => 4,
        GNOME => 4,
        KDE => 4,
        _ => 4,
    }
}

pub fn setup_dbus_interface<T: Send + Sync>(
    cross: &mut RwLockWriteGuard<CrossWrapper>,
) -> dbus_crossroads::IfaceToken<T> {
    cross.register::<T>(INTERFACE, |c: &mut IfaceBuilder<T>| {
        c.method_with_cr_async(
            "GetKeyboardLayouts",
            (),
            ("layouts",),
            move |mut ctx, _, ()| async move { ctx.reply(Ok((get_keyboard_list_backend(),))) },
        );
        c.method_with_cr_async(
            "GetSavedLayouts",
            (),
            ("layouts",),
            move |mut ctx, _, ()| async move { ctx.reply(Ok((get_saved_layouts(),))) },
        );
        c.method_with_cr_async(
            "SaveLayoutOrder",
            ("layouts",),
            (),
            move |mut ctx, _, (layouts,): (Vec<KeyboardLayout>,)| async move {
                write_to_config(layouts);
                ctx.reply(Ok(()))
            },
        );
        c.method_with_cr_async(
            "GetMaxActiveKeyboards",
            (),
            ("max",),
            move |mut ctx, _, ()| async move { ctx.reply(Ok((get_max_active_keyboards(),))) },
        );
    })
}
