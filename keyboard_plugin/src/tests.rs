use adw::NavigationPage;
use glib::{
    object::{Cast, ObjectExt},
    types::StaticType,
};
use gtk::{prelude::WidgetExt, ListBox};
use re_set_lib::utils::plugin::PluginTestError;

use crate::{backend::get_saved_layouts, frontend_data};

#[test]
fn test_hyprland_default_file() {
    use crate::r#const::HYPRLAND_DEFAULT_FILE;
    assert!(HYPRLAND_DEFAULT_FILE
        .to_str()
        .unwrap()
        .contains(".config/reset/keyboard.conf"));
    assert!(HYPRLAND_DEFAULT_FILE.is_file());
}

pub fn check_layouts_in_ui() -> Result<(), PluginTestError> {
    adw::init().expect("Adw failed to initialize");
    let (_, layout_boxes) = frontend_data();
    let layouts_length = get_saved_layouts().len();
    let result = layout_boxes
        .first()
        .ok_or_else(|| PluginTestError::new("No layout boxes found"));
    let nav_view = result?.last_child().unwrap();
    let mut nav_view_child = nav_view.first_child().unwrap();

    while nav_view_child.type_() != NavigationPage::static_type() {
        nav_view_child = nav_view_child.next_sibling().unwrap();
    }

    let nav_page = nav_view_child.downcast_ref::<NavigationPage>().unwrap();
    let temp = nav_page.first_child().unwrap();
    let temp = temp.first_child().unwrap();
    let temp = temp.first_child().unwrap();
    let temp = temp.last_child().unwrap();
    let temp = temp.first_child().unwrap();
    let list_box = temp.downcast_ref::<ListBox>().unwrap();

    if layouts_length > 0 {
        let last_layout = list_box.row_at_index((layouts_length - 1) as i32);

        if last_layout.is_none() {
            return Err(PluginTestError::new("Not all layouts selected"));
        }
    }

    Ok(())
}
