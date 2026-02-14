# Integration test fixtures

Used by tests/integration.rs (run via `cargo test --test integration`).

- minimal.dot: start → exit (trivial)
- test_success_only.dot: start → ok (exec true) → exit
- test_out_error.dot: success + error port routing, fix step
- pre_push_exec_only.dot: pre-push topology, all exec true
- exec_fail_exit.dot: exec false → exit (pipeline reports failure)

All use type=exec with true/false for fast, deterministic tests.
