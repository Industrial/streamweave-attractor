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
  run_run_dot_with_env(args, &[])
}

/// Like run_run_dot but with extra env vars (e.g. ATTRACTOR_EXECUTION_LOG=1 to use sync path).
fn run_run_dot_with_env(args: &[&str], env: &[(&str, &str)]) -> (Vec<u8>, Vec<u8>, bool) {
  let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
  let mut cmd = Command::new(cargo.as_str());
  cmd
    .args(["run", "--bin", "run_dot", "--"])
    .args(args)
    .current_dir(env!("CARGO_MANIFEST_DIR"));
  for (k, v) in env {
    cmd.env(k, v);
  }
  let out = cmd.output().expect("cargo run --bin run_dot");
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
  // Use sync execution path (ATTRACTOR_EXECUTION_LOG=1) so the test completes; the async stream path can hang.
  let path = dot_path("pre_push_exec_only.dot");
  let path_str = path.to_str().expect("path");
  let (stdout, stderr, success) =
    run_run_dot_with_env(&[path_str], &[("ATTRACTOR_EXECUTION_LOG", "1")]);
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
      resume_checkpoint: None,
      resume_already_completed: false,
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
      resume_checkpoint: None,
      resume_already_completed: false,
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
      resume_checkpoint: None,
      resume_already_completed: false,
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
  // Use sync execution path so the test completes; the async stream path can hang.
  let log_path = std::env::temp_dir().join("streamweave_attractor_pre_push_exec.log.json");
  let dot = std::fs::read_to_string(dot_path("pre_push_exec_only.dot")).expect("read");
  let ast = streamweave_attractor::dot_parser::parse_dot(&dot).expect("parse");
  let r = tokio::time::timeout(
    std::time::Duration::from_secs(10),
    streamweave_attractor::run_compiled_graph(
      &ast,
      streamweave_attractor::RunOptions {
        run_dir: None,
        resume_checkpoint: None,
        resume_already_completed: false,
        agent_cmd: None,
        stage_dir: None,
        execution_log_path: Some(log_path),
      },
    ),
  )
  .await
  .expect("run_compiled_graph timed out after 10s")
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
      resume_checkpoint: None,
      resume_already_completed: false,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
    },
  )
  .await
  .expect("run_compiled_graph");
  assert!(format!("{:?}", r.last_outcome.status) != "Success");
}

/// Proves that running the same graph twice in a row does not skip execution when a checkpoint
/// exists from the first run. Checkpoint is only used when --resume is explicitly passed.
///
/// Completed runs do leave a checkpoint when run_dir is set; that is intentional so the next
/// run can use --resume if desired. Without --resume, the graph always runs from start.
#[tokio::test]
async fn integration_lib_two_runs_without_resume_both_run_fully() {
  let run_dir = tempfile::tempdir().expect("tempdir");
  let run_path = run_dir.path();
  let log_path = run_path.join("exec.log.json");
  let dot = std::fs::read_to_string(dot_path("pre_push_exec_only.dot")).expect("read");
  let ast = streamweave_attractor::dot_parser::parse_dot(&dot).expect("parse");

  let r1 = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: Some(run_path),
      resume_checkpoint: None,
      resume_already_completed: false,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: Some(log_path.clone()),
    },
  )
  .await
  .expect("first run");
  assert!(format!("{:?}", r1.last_outcome.status) == "Success");
  assert!(r1.completed_nodes.contains(&"pre_push".to_string()));
  assert!(r1.completed_nodes.contains(&"test_coverage".to_string()));

  assert!(
    log_path.exists(),
    "first run must leave execution log when execution_log_path is set"
  );
  let checkpoint_file = run_path.join(streamweave_attractor::checkpoint_io::CHECKPOINT_FILENAME);
  assert!(
    !checkpoint_file.exists(),
    "when execution_log_path is set, checkpoint is not written (log is single source)"
  );

  let r2 = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: Some(run_path),
      resume_checkpoint: None,
      resume_already_completed: false,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: Some(log_path.clone()),
    },
  )
  .await
  .expect("second run");
  assert!(format!("{:?}", r2.last_outcome.status) == "Success");
  assert!(r2.completed_nodes.contains(&"pre_push".to_string()));
  assert!(r2.completed_nodes.contains(&"test_coverage".to_string()));
  assert_eq!(
    r1.completed_nodes, r2.completed_nodes,
    "second run must execute the full graph, not resume from checkpoint"
  );
}
