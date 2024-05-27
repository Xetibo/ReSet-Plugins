{
  inputs =
    {
      nixpkgs.url = "github:NixOs/nixpkgs/nixos-unstable";
      rust-overlay.url = "github:oxalica/rust-overlay";
      flake-utils.url = "github:numtide/flake-utils";
    };

  outputs = { nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs =
            import
              nixpkgs
              {
                system = "x86_64-linux";
                overlays = [
                  (import rust-overlay)
                ];
              };
        in
        {
          devShell =
            pkgs.mkShell
              {
                nativeBuildInputs = with pkgs; [
                  pkg-config
                  wrapGAppsHook4
                  glib
                  rust-bin.nightly."2024-05-10".complete
                ];

                buildInputs = with pkgs; [
                  dbus
                  gtk4
                  libadwaita
                  pulseaudio
                  xorg.setxkbmap
                  glib
                ];
                LD_LIBRARY_PATH = "${pkgs.glib.out}/lib";
              };
        }
      );
}
