{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    import-cargo.url = github:edolstra/import-cargo;
    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs @ { self, ... }:
    (inputs.flake-utils.lib.eachDefaultSystem (system:
      let

        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [ inputs.rust-overlay.overlays.default ];
        };

        rust-config = {
          extensions = [ "rust-src" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        rust = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain).override rust-config;

        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rust;

        shellDeps = [
          rust
        ] ++ (with pkgs; [
          just
          nixpkgs-fmt
        ]);

        cupcake-cli =
          let
            pname = (craneLib.crateNameFromCargoToml { cargoToml = ./cupcake-cli/Cargo.toml; }).pname;
            version = (craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; }).version;
            craneArgs = rec {
              inherit
                pname
                version
                ;
              src = craneLib.cleanCargoSource (craneLib.path ./.);
              cargoExtraArgs = "-p ${pname}";
              doCheck = false;
            };
            cargoArtifacts = craneLib.buildDepsOnly craneArgs;
          in
          craneLib.buildPackage (craneArgs // {
            inherit cargoArtifacts;
            src = pkgs.lib.cleanSourceWith {
              src = ./.;
              filter =
                let
                  regoFilter = path: _type: builtins.match ".*rego$" path != null;
                in
                path: _type: (regoFilter path _type) || (craneLib.filterCargoSources path _type);
            };
          });

      in
      rec {

        devShells = {
          default = pkgs.mkShell ({
            buildInputs = shellDeps;
          });
        };

        packages = {
          inherit cupcake-cli;
        };

      }));
}
