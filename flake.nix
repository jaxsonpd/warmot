{
  description = "warmot — Rust/Tauri satellite & radar imagery viewer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };

        # Tauri v2 runtime/build deps on Linux (webkitgtk 4.1 stack).
        # If you're building for macOS/Windows from a native host instead,
        # these system libs aren't needed there — Nix is Linux-only for this.
        tauriLinuxDeps = with pkgs; [
          glib
          gtk3
          cairo
          pango
          gdk-pixbuf
          atk
          at-spi2-atk
          at-spi2-core
          webkitgtk_4_1
          libsoup_3
          librsvg
          libayatana-appindicator
          xdotool # provides libxdo, used for global shortcuts
          dbus
          openssl
        ];

        # Uncomment if `core`'s JP2 → raster pipeline links against these
        # (e.g. via jpeg2k-sys or gdal-sys) rather than a pure-Rust decoder.
        # imageryDeps = with pkgs; [ openjpeg gdal ];
        imageryDeps = [ ];
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook3
          ];

          buildInputs = tauriLinuxDeps ++ imageryDeps;

          packages = with pkgs; [
            rustToolchain
            cargo-tauri
            nodejs_22

            # handy extras
            cargo-watch
            cargo-edit
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath tauriLinuxDeps}:$LD_LIBRARY_PATH"
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
            echo "warmot dev shell — rustc $(rustc --version | cut -d' ' -f2), node $(node --version)"
          '';
        };
      });
}