{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci.url = "github:yusdacra/nix-cargo-integration";
    nci.inputs.nixpkgs.follows = "nixpkgs";
    parts.url = "github:hercules-ci/flake-parts";
    parts.inputs.nixpkgs-lib.follows = "nixpkgs";
    devshell.url = "github:numtide/devshell";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs @ {
    parts,
    nci,
    devshell,
    nixpkgs,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      imports = [
        nci.flakeModule
        parts.flakeModules.easyOverlay
        devshell.flakeModule
      ];
      perSystem = {
        config,
        pkgs,
        system,
        lib,
        inputs',
        ...
      }: let
        crateGui = "muzik_gui";
        # shorthand for accessing this crate's outputs
        # you can access crate outputs under `config.nci.outputs.<crate name>` (see documentation)
        crateOutputs = config.nci.outputs.${crateGui};
        binPath = lib.makeBinPath [
          pkgs.yt-dlp
          pkgs.ffmpeg_5-full
          pkgs.sqlite
          pkgs.opusTools
        ];
        guiDeps = [];
      in {
        # declare projects
        # relPath is the relative path of a project to the flake root
        _module.args.pkgs = import nixpkgs {
          inherit system;
          overlays = [inputs.rust-overlay.overlays.default];
        };

        nci.projects.${crateGui}.relPath = "";

        # configure crates
        nci.crates.${crateGui} = {
          # export crate (packages and devshell) in flake outputs
          # alternatively you can access the outputs and export them yourself (see below)
          export = true;
          # look at documentation for more options
          overrides = {
            add-inputs.overrideAttrs = old: {
              nativeBuildInputs =
                (old.nativeBuildInputs or [])
                ++ [
                  pkgs.makeWrapper
                  pkgs.llvmPackages_16.bintools
                ];
              buildInputs = with pkgs;
                (old.buildInputs or [])
                ++ [
                  pkgs.llvmPackages_16.bintools
                  pkgs.pkg-config
                  pkgs.openssl.dev
                  pkgs.openssl
                  pkgs.perl
                  openssl.dev
                  glib.dev
                  gdk-pixbuf.dev
                  pango.dev
                  cairo.dev
                  gtk3.dev
                  harfbuzz.dev
                  # TODO:trim dependencies
                  webkitgtk.dev
                  libayatana-appindicator.dev
                  atk.dev
                  webkitgtk_4_1.dev
                  libsoup_3.dev
                  zlib.dev
                ];
              postInstall = ''
                wrapProgram "$out/bin/${crateGui}" --set PATH ${binPath}
              '';
            };
          };
          depsOverrides = {
            add-inputs.overrideAttrs = old: {
              nativeBuildInputs =
                (old.nativeBuildInputs or [])
                ++ [
                ];
              buildInputs = with pkgs;
                (
                  old.buildInputs
                  or [
                  ]
                )
                ++ [
                  pkgs.pkg-config
                  pkgs.openssl.dev
                  pkgs.openssl
                  pkgs.perl
                  openssl.dev
                  glib.dev
                  gdk-pixbuf.dev
                  pango.dev
                  cairo.dev
                  gtk3.dev
                  harfbuzz.dev
                  webkitgtk.dev
                  libayatana-appindicator.dev
                  atk.dev
                  webkitgtk_4_1.dev
                  libsoup_3.dev
                  zlib.dev
                ];
            };
          };
        };

        nci.toolchains = {
          build = inputs.rust-overlay.packages."${system}".rust;
        };

        # export the crate devshell as the default devshell
        devshells.default = with pkgs; {
          env = [
            /*
            {
            name = "RUST_SRC_PATH";
            value = rustPlatform.rustLibSrc;
            }
            */
            {
              name = "RUST_SRC_PATH";
              value = "${pkgs.rust-bin.stable.latest.default.override {
                extensions = ["rust-src"];
              }}/lib/rustlib/src/rust/library";
            }
            {
              name = "PKG_CONFIG_PATH";
              value = lib.strings.makeSearchPath "lib/pkgconfig" [
                openssl.dev
                zlib.dev
                glib.dev
                gdk-pixbuf.dev
                pango.dev
                cairo.dev
                gtk3.dev
                harfbuzz.dev
                webkitgtk.dev
                libayatana-appindicator.dev
                atk.dev
                webkitgtk_4_1.dev
                libsoup_3.dev
              ];
            }
            {
              name = "LD_LIBRARY_PATH";
              value = lib.makeLibraryPath [
                libxkbcommon
                libGL

                # WINIT_UNIX_BACKEND=wayland
                wayland

                # WINIT_UNIX_BACKEND=x11
                xorg.libXcursor
                xorg.libXrandr
                xorg.libXi
                xorg.libX11
                openssl
              ];
            }
          ];

          packages = [
            (rust-bin.stable.latest.default.override {
              extensions = ["rust-src"];
            })
            cargo-watch
            rust-analyzer
            sea-orm-cli
            just
            pkg-config
            jq
            yt-dlp
            opusTools
            llvmPackages_16.bintools
            mold
            openssl
          ];

          commands = [
            {
              name = "run-tui";
              command = "RUST_LOG=debug nix run . -- tui";
              help = "Run the muzik tui";
              category = "Run";
            }

            {
              name = "run-gui";
              command = "RUST_LOG=debug nix run .#muzik_gui";
              help = "Run the muzik gui";
              category = "Run";
            }

            {
              name = "run-dbtest";
              command = "RUST_LOG=debug nix run . -- db-test";
              help = "Run the dbtest";
              category = "Run";
            }

            {
              name = "lrun-gui";
              command = "RUST_LOG=debug cargo r -p muzik_gui";
              help = "Locally run GUI";
              category = "Local run";
            }

            {
              name = "lrun-gui-skia-release";
              command = "ICED_BACKEND=tiny-skia RUST_LOG=debug cargo r --release -p muzik_gui";
              help = "Locally run GUI, with tiny-skia in release mode";
              category = "Local run";
            }

            {
              name = "build-muzik";
              command = "nix build .";
              help = "build muzik";
              category = "Build";
            }
          ];
        };
        # export the release package of the crate as default package
        packages.default = crateOutputs.packages.release;

        overlayAttrs = {
          inherit (config.packages) muzik_gui;
        };
        packages.muzik_gui = crateOutputs.packages.release;
      };
    };
}
