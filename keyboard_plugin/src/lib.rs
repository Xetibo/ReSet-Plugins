use adw::NavigationView;
use gtk::prelude::*;
use gtk::{Box, Orientation};
use re_set_lib::utils::plugin::{
    PluginCapabilities, PluginImplementation, PluginTestError, PluginTestFunc, SidebarInfo,
};
use tests::check_layouts_in_ui;

use crate::backend::get_saved_layouts;
use crate::frontend::add_layout_page::create_add_keyboard_page;
use crate::frontend::create_title;
use crate::frontend::main_page::create_keyboard_main_page;

mod backend;
mod r#const;
mod frontend;
mod keyboard_layout;
mod tests;
mod utils;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    PluginCapabilities::new(vec!["Keyboard"], true, PluginImplementation::Both)
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_name() -> String {
    String::from("Keyboard")
}

#[no_mangle]
pub extern "C" fn frontend_startup() {
    adw::init().expect("Adw failed to initialize");
}

#[no_mangle]
pub extern "C" fn frontend_shutdown() {}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_data() -> (SidebarInfo, Vec<Box>) {
    let info = SidebarInfo {
        name: "Keyboard",
        icon_name: "input-keyboard-symbolic",
        parent: None,
    };
    let main = Box::builder().orientation(Orientation::Vertical).build();
    main.append(&create_title());

    let nav_view = NavigationView::new();
    main.append(&nav_view);

    create_keyboard_main_page(&nav_view);
    create_add_keyboard_page(&nav_view);

    (info, vec![main])
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn name() -> String {
    String::from("Keyboard")
}

#[no_mangle]
pub extern "C" fn backend_startup() {}

#[no_mangle]
pub extern "C" fn backend_shutdown() {}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_tests() -> Vec<PluginTestFunc> {
    let check_layouts = PluginTestFunc::new(can_get_layouts, "Get layouts");
    let move_ui = PluginTestFunc::new(check_layouts_in_ui, "Check layouts in UI");
    let vec1 = vec![check_layouts, move_ui];
    vec1
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn backend_tests() -> Vec<PluginTestFunc> {
    vec![]
}

fn can_get_layouts() -> Result<(), PluginTestError> {
    let layouts = get_saved_layouts();
    if layouts.is_empty() {
        return Err(PluginTestError::new("No layouts found"));
    }
    Ok(())
}
