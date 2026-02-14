# streamweave-attractor

An implementation of [StrongDM's Attractor](https://github.com/strongdm/attractor)—a non-interactive coding agent for use in a software factory—built on [StreamWeave](https://github.com/Industrial/streamweave).

**Attractor** is specified by [NLSpecs](https://github.com/strongdm/attractor#terminology) (Natural Language Specs): human-readable specs intended for coding agents to implement and validate behavior. This repository implements that spec using **StreamWeave**, a composable, async, stream-first Rust framework for building data processing graphs.

## Overview

- **Attractor** ([strongdm/attractor](https://github.com/strongdm/attractor)): Defines the coding agent loop, unified LLM interface, and overall behavior for a software factory agent.
- **StreamWeave** ([Industrial/streamweave](https://github.com/Industrial/streamweave)): Provides the runtime—graph-based, async stream processing in Rust—used to run the agent loop, tool calls, and I/O.

Together, this project gives you an Attractor-compliant agent implemented in Rust with StreamWeave’s streaming and graph abstractions.

## Specs (upstream)

The authoritative Attractor specs live in the upstream repo:

- [Attractor Specification](https://github.com/strongdm/attractor/blob/main/attractor-spec.md)
- [Coding Agent Loop Specification](https://github.com/strongdm/attractor/blob/main/coding-agent-loop-spec.md)
- [Unified LLM Client Specification](https://github.com/strongdm/attractor/blob/main/unified-llm-spec.md)

## Building and running

### Running via CLI

**`run_dot`** runs an Attractor pipeline from a `.dot` file (parse → validate → run):

```bash
devenv shell -- cargo run --bin run_dot -- examples/workflows/pre-push.dot
```

Options (see `run_dot --help` for full usage): **`--agent-cmd`**, **`--stage-dir`** (default: `.attractor`), **`--run-dir`**, **`--resume`**. A checkpoint is written to the run directory (default `.attractor/checkpoint.json`) on successful exit only (no mid-run crash recovery). Use **`--run-dir DIR`** to write the checkpoint to `DIR/checkpoint.json`, and **`--resume DIR`** to resume from `DIR/checkpoint.json` with the same .dot file.

**Environment variables:**

- **`ATTRACTOR_AGENT_CMD`** — Command for agent/codergen nodes (e.g. `cursor-agent`). When set, agent steps run this with the prompt as stdin; outcome is read from `ATTRACTOR_STAGE_DIR`.
- **`ATTRACTOR_STAGE_DIR`** — Directory for agent `outcome.json` and staging (default: `.attractor`).

Example:

```bash
devenv shell -- cargo run --bin run_dot -- examples/workflows/pre-push.dot
```

The main entry point is the `simple_pipeline` example, which runs a minimal Attractor graph (Run tests → Report). You can run it in several ways:

| Method | Command | Notes |
|--------|---------|-------|
| **Nix (remote)** | `nix run github:Industrial/streamweave-attractor` | Fetches, builds, and runs in one step |
| **Nix (local)** | `nix run` | From a local checkout; builds the example and installs as `streamweave-attractor` |
| **Cargo** | `cargo run --example simple_pipeline` | Direct run; requires Rust toolchain |
| **devenv** | `devenv shell -- cargo run --example simple_pipeline` | Uses project devenv for consistent tooling |

**Examples:**

```bash
# Run from Nix (no Rust install needed)
nix run github:Industrial/streamweave-attractor

# Run from local checkout with devenv
devenv shell -- cargo run --example simple_pipeline

# Build only
devenv shell -- cargo build --examples
```

### Nix flake (Cargo.lock and path dependency)

- **Cargo.lock** is committed so that when the repo is used as a flake input (e.g. `github:Industrial/streamweave-attractor`), Nix can use it for reproducible builds. Do not add `Cargo.lock` to `.gitignore`.
- The flake depends on **streamweave** via input `streamweave` (default: `path:../streamweave`). When the flake is consumed from GitHub (e.g. by another project’s `devenv`), that path does not exist. Override the input in the consuming flake, for example:
  ```nix
  streamweave-attractor.url = "github:Industrial/streamweave-attractor";
  # Override so the build can find streamweave:
  streamweave-attractor.inputs.streamweave.url = "github:Industrial/streamweave";
  # or path: ./streamweave if you have it in your tree
  ```
- **[cargo2nix](https://github.com/cargo2nix/cargo2nix)** is integrated optionally. The default package uses `buildRustPackage`. To use cargo2nix (granular Rust builds), generate `Cargo.nix` once and commit it:
  ```bash
  nix run .#generate
  git add Cargo.nix
  ```
  Then `nix build .#cargo2nix` builds the workspace crate via cargo2nix. The default `nix build` / `nix run` still use `buildRustPackage` and do not require `Cargo.nix`.

### Development

```bash
devenv shell -- cargo build
devenv shell -- cargo run --example simple_pipeline
```

### Pre-push quality gates

Run all quality checks before pushing:

```bash
devenv shell -- bin/pre-push
```

This runs: format, fix, check, lint, build, test, audit, check-docs. (Examples like `simple_pipeline` require an LLM—run `bin/test-examples` manually when needed.) See `examples/workflows/pre-push.dot` for the bd-centric fix-and-retry workflow.

See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) for development workflow and agent instructions.

## Installing as a CLI in another project (devenv)

You can install `streamweave-attractor` as a CLI in another project’s devenv environment so the `streamweave-attractor` binary is available in the shell.

### 1. Add the flake input

In the **other** project, add `streamweave-attractor` as an input.

**If that project uses a `devenv.yaml`** (standalone devenv), add:

```yaml
# devenv.yaml
inputs:
  streamweave-attractor:
    url: github:Industrial/streamweave-attractor
  # Optional: use a local checkout instead of GitHub
  # streamweave-attractor:
  #   path: ../streamweave-attractor
```

**If that project uses a Nix flake** (e.g. `flake.nix` with devenv), add the same input to the flake’s `inputs` and ensure it is passed through to the devenv module (e.g. via `inputs` in `perSystem` or your devenv integration).

### 2. Add the package in `devenv.nix`

In the other project’s `devenv.nix`, add the package so it is on `PATH`:

```nix
{ inputs, pkgs, ... }: {
  packages = with pkgs; [
    # your other packages ...
    inputs.streamweave-attractor.packages.${pkgs.system}.default
  ];
}
```

After `devenv shell` (or entering the environment), you can run:

```bash
streamweave-attractor
```

The flake’s default package builds the `simple_pipeline` example and installs it as the `streamweave-attractor` binary. For `run_dot` (DOT-based pipelines), use this repo’s devenv and run:

```bash
cargo run --bin run_dot -- examples/workflows/pre-push.dot
```

## License

This project is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/). See [LICENSE](LICENSE) for details.
