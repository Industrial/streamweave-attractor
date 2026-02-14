{
  description = "Attractor pipeline as a StreamWeave graph";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix/release-0.12";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    cargo2nix,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [cargo2nix.overlays.default];
      };
      # Default build: buildRustPackage (streamweave from crates.io 0.10.0).
      # Cargo.lock must be committed so the flake input has it (see README).
      defaultPackage = pkgs.rustPlatform.buildRustPackage {
        pname = "streamweave-attractor";
        version = "0.2.0";
        src = self;
        cargoLock.lockFile = self + "/Cargo.lock";
        nativeBuildInputs = [pkgs.pkg-config];
        buildInputs = [pkgs.openssl];
        cargoBuildFlags = ["--example" "simple_pipeline"];
        installPhase = ''
          runHook preInstall
          # buildRustPackage build phase may not build the example; build it here so it exists
          cargo build --release --example simple_pipeline
          mkdir -p $out/bin
          cp target/release/examples/simple_pipeline $out/bin/streamweave-attractor
          runHook postInstall
        '';
      };
      # cargo2nix build (optional): only when Cargo.nix exists.
      # Generate with: nix run .#generate  (or nix run github:cargo2nix/cargo2nix -- cargo2nix)
      hasCargoNix = pkgs.lib.pathExists (self + "/Cargo.nix");
      rustPkgs =
        if hasCargoNix
        then
          pkgs.rustBuilder.makePackageSet {
            packageFun = import (self + "/Cargo.nix");
            workspaceSrc = self;
            rustVersion = "1.83.0";
            packageOverrides = pkgs: pkgs.rustBuilder.overrides.all;
          }
        else null;
      cargo2nixPackage =
        if rustPkgs != null
        then rustPkgs.workspace.streamweave-attractor {}
        else null;
    in {
      packages =
        {
          default = defaultPackage;
          # buildRustPackage (always available)
          buildRustPackage = defaultPackage;
        }
        // pkgs.lib.optionalAttrs (cargo2nixPackage != null) {
          # cargo2nix workspace package (requires Cargo.nix in repo)
          cargo2nix = cargo2nixPackage;
        };

      apps = {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/streamweave-attractor";
        };
        # Generate Cargo.nix (run once, then commit Cargo.nix)
        generate = {
          type = "app";
          program = toString (pkgs.writers.writeBash "generate-cargo-nix" ''
            set -e
            nix run github:cargo2nix/cargo2nix/release-0.12 -- cargo2nix
            echo "Generated Cargo.nix - review and commit it."
          '');
        };
      };
    });
}
