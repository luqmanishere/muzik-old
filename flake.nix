{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci.url = "github:yusdacra/nix-cargo-integration";
    nci.inputs.nixpkgs.follows = "nixpkgs";
    parts.url = "github:hercules-ci/flake-parts";
    parts.inputs.nixpkgs-lib.follows = "nixpkgs";
    devshell.url = "github:numtide/devshell";
  };

  outputs =
    inputs @ { parts
    , nci
    , devshell
    , nixpkgs
    , ...
    }:
    parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      imports = [
        nci.flakeModule
        parts.flakeModules.easyOverlay
        devshell.flakeModule
      ];
      perSystem = { config, pkgs, system, inputs', ... }:
        let
          crateName = "muzik";
          # shorthand for accessing this crate's outputs
          # you can access crate outputs under `config.nci.outputs.<crate name>` (see documentation)
          crateOutputs = config.nci.outputs.${crateName};
        in
        {
          # declare projects
          # relPath is the relative path of a project to the flake root
          # TODO: change this to your crate's path
          nci.projects.${crateName}.relPath = "";
          # configure crates
          nci.crates.${crateName} = {
            # export crate (packages and devshell) in flake outputs
            # alternatively you can access the outputs and export them yourself (see below)
            export = true;
            # look at documentation for more options
            overrides = {
              add-inputs.overrideAttrs = old: {
                nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ ];
                buildInputs = (old.buildInputs or [ ]) ++ [ pkgs.pkg-config pkgs.openssl.dev pkgs.openssl pkgs.perl ];
              };
            };
            depsOverrides = {
              add-inputs.overrideAttrs = old: {
                nativeBuildInputs = (old.nativeBuildInputs or [ ]) ++ [ ];
                buildInputs = (old.buildInputs or [ ]) ++ [ pkgs.pkg-config pkgs.openssl.dev pkgs.openssl pkgs.perl ];
              };
            };
          };
          # export the crate devshell as the default devshell
          devshells.default = with pkgs; {
            motd = ''
              muzik devshell
            '';
            env = [
              { name = "RUST_SRC_PATH"; value = rustPlatform.rustLibSrc; }
            ];

            packages = [
              cargo
              rust-analyzer
              rustc
              rustfmt
              just
            ];
          };
          # export the release package of the crate as default package
          packages.default = crateOutputs.packages.release;

          overlayAttrs = {
            inherit (config.packages) muzik;
          };
          packages.muzik = crateOutputs.packages.release;
        };
    };
}
