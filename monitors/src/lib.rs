use re_set_lib::utils::plugin::{PluginCapabilities, PluginImplementation};
use utils::check_environment_support;

pub mod backend;
pub mod r#const;
pub mod frontend;
pub mod utils;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    if check_environment_support() {
        return PluginCapabilities::new(vec!["Monitors"], true, PluginImplementation::Both);
    }
    PluginCapabilities::new(vec![], true, PluginImplementation::Both)
}
