{
  description = "Official plugins for the ReSet settings application.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs @ { self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem =
        { config
        , self'
        , inputs'
        , pkgs
        , system
        , ...
        }:
        {
          _module.args.pkgs = import self.inputs.nixpkgs {
            inherit system;
            overlays = [
              (import
                inputs.rust-overlay
              )
            ];
          };
          devShells.default = pkgs.mkShell {
            inputsFrom = builtins.attrValues self'.packages;
            packages = with pkgs; [
              # (rust-bin.selectLatestNightlyWith
              # (toolchain: toolchain.default))
              rust-bin.nightly."2024-05-10".default
              rust-analyzer
              clippy
              llvmPackages.clang
              libxkbcommon
              glib
              pkg-config
              xorg.setxkbmap
              libclang
            ];
            LD_LIBRARY_PATH = "${pkgs.glib}/lib";
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          };

          packages =
            {
              monitor = pkgs.callPackage ./nix/monitor.nix { inherit inputs; };
              keyboard = pkgs.callPackage ./nix/keyboard.nix { inherit inputs; };
            };
        };
    };
}

