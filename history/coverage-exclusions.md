# Coverage exclusions

- **graph.rs (0%)**: Lives in the `streamweave` dependency, not this repo. cargo-llvm-cov excludes `^{CARGO_HOME}(registry|git)` by default. To exclude explicitly: `cargo llvm-cov --ignore-filename-regex streamweave`.
- **agent_run.rs**: Extra tests added for `read_outcome_json` (invalid JSON, missing context_updates). `run_agent` spawns an external process; cover via integration or exclude in CI if needed.
- **fix_node.rs**: Covered by `nodes::fix_node_test` (node_trait_methods, node_execute_forwards_one_item).
