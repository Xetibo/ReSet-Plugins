#[macro_use]
mod utils;
pub mod backend;
pub mod r#const;
pub mod frontend;
#[cfg(test)]
mod tests;

use re_set_lib::utils::plugin::{PluginCapabilities, PluginImplementation};
use utils::check_environment_support;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    if check_environment_support() {
        return PluginCapabilities::new(vec!["Monitors"], true, PluginImplementation::Both);
    }
    PluginCapabilities::new(vec![], true, PluginImplementation::Both)
}
