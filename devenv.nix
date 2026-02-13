{
  inputs,
  pkgs,
  ...
}: {
  # Name of the project with version
  name = "streamweave-attractor";

  # Languages
  languages = {
    javascript = {
      enable = true;
      bun = {
        enable = true;
      };
    };

    rust = {
      enable = true;
      channel = "stable";
      components = [
        "cargo"
        "clippy"
        "rust-analyzer"
        "rustc"
        "rustfmt"
        "llvm-tools"
      ];
      targets = [];
    };
  };

  env = {
    RUST_BACKTRACE = "1";
    CARGO_TERM_COLOR = "always";
  };

  # Development packages
  packages = with pkgs; [
    # AI
    inputs.nixpkgs-unstable.legacyPackages.${pkgs.system}.beads

    # Rust tools
    clippy
    rust-analyzer
    rustc

    # Development tools
    direnv
    # Git hooks (prek = pre-commit replacement, single binary, no Python)
    prek

    # Formatting tools
    alejandra

    # Publishing tools
    cargo-watch
    cargo-audit
    cargo-llvm-cov
    cargo-nextest

    # Version management
    git
    gh

    # treefmt
    actionlint
    alejandra
    beautysh
    biome
    deadnix
    rustfmt
    taplo
    treefmt
    vulnix
    yamlfmt
  ];

  scripts = {
    prek-install = {
      exec = ''
        prek install -q --overwrite
      '';
    };
  };

  enterShell = ''
    prek-install
  '';
}
