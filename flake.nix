{
  description = "Attractor pipeline as a StreamWeave graph";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "streamweave-attractor";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        cargoBuildFlags = ["--example" "simple_pipeline"];
        installPhase = ''
          runHook preInstall
          mkdir -p $out/bin
          cp target/release/examples/simple_pipeline $out/bin/streamweave-attractor
          runHook postInstall
        '';
      };
      apps.default = {
        type = "app";
        program = "${self.packages.${system}.default}/bin/streamweave-attractor";
      };
    });
}
