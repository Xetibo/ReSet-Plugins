{}:
let
  pkgs = import (fetchTarball https://nixos.org/channels/nixos-unstable/nixexprs.tar.xz) { };
in
pkgs.mkShell
{
  nativeBuildInputs = [
    pkgs.pkg-config
    pkgs.wrapGAppsHook4
    pkgs.glib
  ];

  buildInputs = [
    pkgs.dbus
    pkgs.gtk4
    pkgs.libadwaita
    pkgs.pulseaudio
    pkgs.xorg.setxkbmap
    pkgs.glib
  ];
  LD_LIBRARY_PATH = "${pkgs.glib}/lib";
}
