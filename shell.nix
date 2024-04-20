{ pkgs ? import <nixpkgs> { } }:

with pkgs;
mkShell
{
  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = with llvmPackages;[
    dbus
    gtk4
    libadwaita
    pulseaudio
    clang
    libclang.lib
    libxkbcommon
  ];
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
}


#{
#  inputs = {
#    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
#    xkbcommon-sys = {
#      url = "github:meh/rust-xkbcommon-sys";
#      inputs.nixpkgs.follows = "nixpkgs";
#    };
#  };
#  outputs = inputs:
#    let
#      pkgs = import inputs.nixpkgs {
#        system = "x86_64-linux";
#      };
#    in
#    {
#      devShell = pkgs.mkShell {
#        buildInputs = with pkgs; [
#          dbus
#          gtk4
#          libadwaita
#          pulseaudio
#          libxkbcommon
#          clang
#          xorg.libX11
#          xorg.xkbcomp
#          xorg.xkbutils
#          xkbcommon-sys
#          xorg.libxcb
#          libclang.lib
#        ];
#      };
#    };
#}
