{ pkgs ? import <nixpkgs> { } }:
with pkgs;
mkShell
{
  nativeBuildInputs = [
    pkg-config
    libxkbcommon
    clang
    libclang
  ];

  buildInputs = [
    dbus
    gtk4
    libadwaita
    pulseaudio
    llvmPackages.libclang
  ];
  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";

}
