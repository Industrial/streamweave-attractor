//! Integration tests for the run_dot CLI and compiled graph path.
//!
//! - CLI: runs the binary via `cargo run --bin run_dot` with temp .dot files.
//! - Compiled path: pre-push.dot via run_compiled_graph (library).

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

/// Runs a pre-push-shaped workflow (same topology as pre-push.dot) with quick exec commands
/// so the test finishes in reasonable time. Verifies run_compiled_graph end-to-end.
#[tokio::test]
async fn pre_push_dot_via_run_compiled_graph() {
  let dot = r#"
    digraph PrePush {
      graph [goal="test"]
      rankdir=LR
      start [shape=Mdiamond]
      exit [shape=Msquare]
      pre_push [type=exec, command="true"]
      test_coverage [type=exec, command="true"]
      fix_pre_push [label="Fix"]
      fix_test_coverage [label="Fix"]
      start -> pre_push
      pre_push -> test_coverage [condition="outcome=success"]
      pre_push -> fix_pre_push [condition="outcome=fail"]
      test_coverage -> exit [condition="outcome=success"]
      test_coverage -> fix_test_coverage [condition="outcome=fail"]
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let result = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      resume_checkpoint: None,
    },
  )
  .await
  .expect("run_compiled_graph");
  assert!(
    !result.context.is_empty() || result.last_outcome.notes.is_some(),
    "expected context or outcome notes"
  );
}

/// Test out/error port wiring: exec that fails sends to error port → fix node → exit.
#[tokio::test]
async fn test_out_error_dot_error_path_then_fix_to_exit() {
  let dot = r#"
    digraph TestOutError {
      graph [goal="test out/error ports"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      ok [type=exec, command="true"]
      fail_step [type=exec, command="false"]
      fix [type=exec, command="true"]
      start -> ok
      ok -> fail_step
      fail_step -> exit [condition="outcome=success"]
      fail_step -> fix [condition="outcome=fail"]
      fix -> exit
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let result = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      resume_checkpoint: None,
    },
  )
  .await
  .expect("run_compiled_graph");
  assert!(
    result.completed_nodes.contains(&"fix".to_string()),
    "fix node should run (error port from fail_step → fix → exit); completed: {:?}",
    result.completed_nodes
  );
  assert!(
    result.completed_nodes.contains(&"fail_step".to_string()),
    "fail_step should complete (then error path to fix); completed: {:?}",
    result.completed_nodes
  );
}

/// Run pipeline with --run-dir, then resume from that checkpoint via library.
#[tokio::test]
async fn run_dir_writes_checkpoint_resume_completes() {
  let dot = r#"
    digraph G {
      graph [goal="resume-test"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let run_dir = tempfile::tempdir().expect("temp dir");

  streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: Some(run_dir.path()),
      resume_checkpoint: None,
    },
  )
  .await
  .expect("run_compiled_graph");

  let cp_path = run_dir.path().join("checkpoint.json");
  assert!(
    cp_path.exists(),
    "checkpoint.json should exist after run with run_dir"
  );
  let cp =
    streamweave_attractor::checkpoint_io::load_checkpoint(&cp_path).expect("load checkpoint");
  assert_eq!(
    cp.context.get("goal").map(String::as_str),
    Some("resume-test")
  );
  assert!(!cp.completed_nodes.is_empty() || !cp.current_node_id.is_empty());

  let resumed = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      resume_checkpoint: Some(&cp),
    },
  )
  .await
  .expect("run_compiled_graph resume");
  assert!(
    format!("{:?}", resumed.last_outcome.status) == "Success",
    "resumed run should complete successfully"
  );
}
