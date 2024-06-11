#[test]
fn test_hyprland_default_file() {
    use crate::r#const::HYPRLAND_DEFAULT_FILE;
    assert!(HYPRLAND_DEFAULT_FILE
        .to_str()
        .unwrap()
        .contains(".config/reset/keyboard.conf"));
    assert!(HYPRLAND_DEFAULT_FILE.is_file());
}
