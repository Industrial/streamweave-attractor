//! Integration tests for the run_dot CLI.
//!
//! Runs the binary via `cargo run --bin run_dot` with temp .dot files.

use std::process::Command;

fn run_run_dot(args: &[&str]) -> std::process::Output {
  Command::new("cargo")
    .args(["run", "--bin", "run_dot", "--"])
    .args(args)
    .output()
    .expect("run cargo run --bin run_dot")
}

#[test]
fn run_dot_prints_usage_without_args() {
  let out = run_run_dot(&[]);
  assert!(!out.status.success());
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(stderr.contains("Usage") || stderr.contains("usage"));
  assert!(stderr.contains("run_dot") || stderr.contains(".dot"));
}

#[test]
fn run_dot_exits_1_for_missing_file() {
  let out = run_run_dot(&["/nonexistent/path.dot"]);
  assert!(!out.status.success());
  let stderr = String::from_utf8_lossy(&out.stderr);
  assert!(
    stderr.contains("Error") || stderr.contains("error") || stderr.contains("reading"),
    "stderr: {}",
    stderr
  );
}

#[test]
fn run_dot_succeeds_with_minimal_start_exit_dot() {
  let dir = tempfile::tempdir().expect("temp dir");
  let path = dir.path().join("minimal.dot");
  std::fs::write(
    &path,
    r#"digraph G {
  graph [goal="test"]
  start [shape=Mdiamond]
  exit [shape=Msquare]
  start -> exit
}"#,
  )
  .expect("write dot");

  let path_str = path.to_str().expect("path");
  let out = run_run_dot(&[path_str]);
  assert!(
    out.status.success(),
    "stderr: {} stdout: {}",
    String::from_utf8_lossy(&out.stderr),
    String::from_utf8_lossy(&out.stdout)
  );
  let stdout = String::from_utf8_lossy(&out.stdout);
  assert!(stdout.contains("Pipeline completed"));
  assert!(stdout.contains("Success") || stdout.contains("completed"));
}
