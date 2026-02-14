//! Integration tests that run the run_dot CLI and/or run_compiled_graph on .dot fixtures
//! in tests/integration/. These give full coverage of parsing, compilation, exec/codergen
//! routing, and CLI so we can refactor safely.

use std::path::Path;
use std::process::Command;

fn integration_dir() -> std::path::PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("tests")
    .join("integration")
}

fn dot_path(name: &str) -> std::path::PathBuf {
  integration_dir().join(name)
}

/// Run `cargo run --bin run_dot -- <args...>` from the crate root. Returns (stdout, stderr, success).
fn run_run_dot(args: &[&str]) -> (Vec<u8>, Vec<u8>, bool) {
  let out = Command::new("cargo")
    .args(["run", "--bin", "run_dot", "--"])
    .args(args)
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .output()
    .expect("cargo run --bin run_dot");
  (out.stdout, out.stderr, out.status.success())
}

// ---- CLI tests using tests/integration/*.dot ----

#[test]
fn integration_minimal_dot_succeeds() {
  let path = dot_path("minimal.dot");
  let path_str = path.to_str().expect("path");
  let (stdout, stderr, success) = run_run_dot(&[path_str]);
  assert!(
    success,
    "minimal.dot should succeed: stderr={}",
    String::from_utf8_lossy(&stderr)
  );
  let out = String::from_utf8_lossy(&stdout);
  assert!(out.contains("Pipeline completed"));
  assert!(out.contains("Success"));
}

#[test]
fn integration_test_success_only_dot_succeeds() {
  let path = dot_path("test_success_only.dot");
  let path_str = path.to_str().expect("path");
  let (stdout, stderr, success) = run_run_dot(&[path_str]);
  assert!(
    success,
    "test_success_only.dot should succeed: stderr={}",
    String::from_utf8_lossy(&stderr)
  );
  let out = String::from_utf8_lossy(&stdout);
  assert!(out.contains("Pipeline completed"));
  assert!(out.contains("Success"));
}

#[test]
fn integration_test_out_error_dot_succeeds() {
  // fail_step fails -> fix (true) -> exit: pipeline still completes with Success
  let path = dot_path("test_out_error.dot");
  let path_str = path.to_str().expect("path");
  let (stdout, stderr, success) = run_run_dot(&[path_str]);
  assert!(
    success,
    "test_out_error.dot should succeed: stderr={}",
    String::from_utf8_lossy(&stderr)
  );
  let out = String::from_utf8_lossy(&stdout);
  assert!(out.contains("Pipeline completed"));
  assert!(out.contains("Success"));
  assert!(out.contains("fix") || out.contains("fail_step") || out.contains("Completed nodes"));
}

#[test]
fn integration_pre_push_exec_only_dot_succeeds() {
  let path = dot_path("pre_push_exec_only.dot");
  let path_str = path.to_str().expect("path");
  let (stdout, stderr, success) = run_run_dot(&[path_str]);
  assert!(
    success,
    "pre_push_exec_only.dot should succeed: stderr={}",
    String::from_utf8_lossy(&stderr)
  );
  let out = String::from_utf8_lossy(&stdout);
  assert!(out.contains("Pipeline completed"));
  assert!(out.contains("Success"));
}

#[test]
fn integration_exec_fail_exit_dot_fails() {
  // Exec false -> exit on fail path: pipeline reports failure, run_dot exits non-zero
  let path = dot_path("exec_fail_exit.dot");
  let path_str = path.to_str().expect("path");
  let (stdout, _stderr, success) = run_run_dot(&[path_str]);
  assert!(!success, "exec_fail_exit.dot should fail (exit non-zero)");
  let out = String::from_utf8_lossy(&stdout);
  assert!(out.contains("Pipeline completed"));
  assert!(out.contains("Fail") || out.contains("Error") || out.contains("failure"));
}

// ---- Library path: run_compiled_graph on same graphs ----

#[tokio::test]
async fn integration_lib_minimal_succeeds() {
  let dot = std::fs::read_to_string(dot_path("minimal.dot")).expect("read minimal.dot");
  let ast = streamweave_attractor::dot_parser::parse_dot(&dot).expect("parse");
  let r = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
    },
  )
  .await
  .expect("run_compiled_graph");
  assert!(format!("{:?}", r.last_outcome.status) == "Success");
  assert!(
    r.completed_nodes
      .iter()
      .any(|n| n == "exit" || n == "start")
  );
}

#[tokio::test]
async fn integration_lib_test_success_only_succeeds() {
  let dot = std::fs::read_to_string(dot_path("test_success_only.dot")).expect("read");
  let ast = streamweave_attractor::dot_parser::parse_dot(&dot).expect("parse");
  let r = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
    },
  )
  .await
  .expect("run_compiled_graph");
  assert!(format!("{:?}", r.last_outcome.status) == "Success");
  assert!(r.completed_nodes.contains(&"ok".to_string()));
}

#[tokio::test]
async fn integration_lib_test_out_error_succeeds() {
  let dot = std::fs::read_to_string(dot_path("test_out_error.dot")).expect("read");
  let ast = streamweave_attractor::dot_parser::parse_dot(&dot).expect("parse");
  let r = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
    },
  )
  .await
  .expect("run_compiled_graph");
  assert!(format!("{:?}", r.last_outcome.status) == "Success");
  assert!(r.completed_nodes.contains(&"fix".to_string()));
  assert!(r.completed_nodes.contains(&"fail_step".to_string()));
}

#[tokio::test]
async fn integration_lib_pre_push_exec_only_succeeds() {
  let dot = std::fs::read_to_string(dot_path("pre_push_exec_only.dot")).expect("read");
  let ast = streamweave_attractor::dot_parser::parse_dot(&dot).expect("parse");
  let r = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
    },
  )
  .await
  .expect("run_compiled_graph");
  assert!(format!("{:?}", r.last_outcome.status) == "Success");
  assert!(r.completed_nodes.contains(&"pre_push".to_string()));
  assert!(r.completed_nodes.contains(&"test_coverage".to_string()));
}

#[tokio::test]
async fn integration_lib_exec_fail_exit_returns_failure() {
  let dot = std::fs::read_to_string(dot_path("exec_fail_exit.dot")).expect("read");
  let ast = streamweave_attractor::dot_parser::parse_dot(&dot).expect("parse");
  let r = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
    },
  )
  .await
  .expect("run_compiled_graph");
  assert!(format!("{:?}", r.last_outcome.status) != "Success");
}
