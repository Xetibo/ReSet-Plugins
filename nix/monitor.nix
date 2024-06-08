{ rustPlatform
, rust-bin
, pkg-config
, wrapGAppsHook4
, gtk4
, gtk4-layer-shell
, libadwaita
, dbus
, pulseaudio
, xorg
, lib
, ...
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ../monitors/Cargo.toml);
  lockFile = ../monitors/Cargo.lock;
in
rustPlatform.buildRustPackage rec {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;

  src = ../monitors/.;

  buildInputs = [
    pkg-config
    gtk4
    gtk4-layer-shell
    libadwaita
    dbus
    pulseaudio
    xorg.libXrandr
  ];

  cargoLock = {
    # outputHashes = {
    #   "re_set-lib-3.3.0" = "sha256-f+0+rrM+Z0sOXNwYtJxrlcK6wGFbdwamU0sNUm2ennM=";
    # };
    inherit lockFile;
  };

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
    rust-bin.nightly."2024-05-10".default
  ];

  copyLibs = true;

  meta = with lib; {
    description = "A monitor configuration plugin for the ReSet settings application.";
    homepage = "https://github.com/DashieTM/ReSet-Plugins";
    changelog = "https://github.com/DashieTM/ReSet-Plugins/releases/tag/${version}";
    license = licenses.gpl3;
    maintainers = with maintainers; [ DashieTM ];
    mainProgram = "ReSet-Monitor";
  };
}
