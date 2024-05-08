{ pkgs ? import <nixpkgs> {} }:

with pkgs;
mkShell {
{ pkgs ? import <nixpkgs> { } }:

with pkgs;
mkShell
{
  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    dbus
    gtk4
    libadwaita
    pulseaudio
  ];

}