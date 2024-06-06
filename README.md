# ReSet-Plugins

This repository features a list of plugins directly developed by the authors of ReSet.
Specific features and limitations can be found within the respective plugin directory.

## Plugin List

- [Monitors Plugin](monitors/README.md)
- [Keyboard Plugin](keyboard_plugin/README.md)

## Installation

### Confirmation of plugins

In order for your plugins to load, you have to define them in `$xdg_config_dir/reset/ReSet.toml`.
This is done to avoid loading of arbitrary plugins that might be placed within this folder by accident.

### Manual compilation

Compile the source for the chosen plugin by cloning the repository and building the plugin.
After this, simply compile the plugin and move it to the ReSet plugins folder in your `.config/reset/plugins` directory.

### Arch Linux

ReSet provides compiled binaries for both the application and the plugins.
By installing these binaries, the library will automatically be placed within the correct place for a default installation.

### Ubuntu 24.04

ReSet provides installation of binaries for the latest Ubuntu release.
Simply download the Debian packages and install them locally with apt.
This places the chosen plugin within the standard installation path.

### NixOS

ReSet offers a home manager module which can be used to define plugins declaratively.
Please visit [the ReSet main application](https://github.com/Xetibo/ReSet) for documentation of the home manager module.

### Flatpak

Flatpak does not allow ReSet to ship plugins directly, therefore, you would be required to download the compiled binaries within this repository and manually copy them to the plugin directory as defined in [Confirmation](### Confirmation of plugins)
