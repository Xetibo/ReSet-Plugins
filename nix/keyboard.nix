{ rustPlatform
, rust-bin
, pkg-config
, wrapGAppsHook4
, gtk4
, gtk4-layer-shell
, libadwaita
, dbus
, xorg
, pulseaudio
, lib
, ...
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../keyboard_plugin/Cargo.toml);
  lockFile = ../keyboard_plugin/Cargo.lock;
in
rustPlatform.buildRustPackage rec {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = ../keyboard_plugin;

  buildInputs = [
    pkg-config
    gtk4
    gtk4-layer-shell
    libadwaita
    dbus
    xorg.setxkbmap
    pulseaudio
  ];

  cargoLock = {
    outputHashes = {
      "re_set-lib-3.4.1" = "";
    };
    inherit lockFile;
  };

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
    # (rust-bin.selectLatestNightlyWith
    # (toolchain: toolchain.default))
    rust-bin.nightly."2024-05-10".default
  ];

  copyLibs = true;

  meta = with lib; {
    description = "A keyboard configuration plugin for the ReSet settings application.";
    homepage = "https://github.com/DashieTM/ReSet-Plugins";
    changelog = "https://github.com/DashieTM/ReSet-Plugins/releases/tag/${version}";
    license = licenses.gpl3;
    maintainers = with maintainers; [ DashieTM ];
    mainProgram = "ReSet-Keyboard";
  };
}
