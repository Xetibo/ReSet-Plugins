use std::process::Output;

pub fn parse_setting(command_output: Output) -> Vec<String> {
    let output = String::from_utf8(command_output.stdout).expect("not utf8");
    let output = output.lines().next().unwrap();
    let output = output.replace("str:", "");
    let output: Vec<String> = output.split(",").map(|s| s.to_string()).collect();
    output
}

pub fn get_default_path() -> String {
    let dirs = directories_next::ProjectDirs::from("org", "Xetibo", "ReSet")
        .unwrap();
    let buf = dirs.config_dir()
        .join("keyboard.conf");
    let path = buf
        .to_str()
        .unwrap();
    String::from(path)
}
