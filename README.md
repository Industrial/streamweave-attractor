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

## License

This project is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/). See [LICENSE](LICENSE) for details.
