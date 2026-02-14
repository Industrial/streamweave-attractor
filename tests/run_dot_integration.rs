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
      agent_cmd: None,
      stage_dir: None,
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
      agent_cmd: None,
      stage_dir: None,
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

/// Run pipeline with run_dir and verify checkpoint is written.
#[tokio::test]
async fn run_dir_writes_checkpoint() {
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
      agent_cmd: None,
      stage_dir: None,
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
}

// --- TDD: one-shot sender must be dropped so stream closes and graph completes ---
//
// Assumption: When a node sends exactly one item on a port (success or error), it must
// drop that port's sender after the send. Otherwise the downstream (merge/exit) never
// sees the stream close, recv() blocks, the node never finishes, and wait_for_completion() hangs.
//
// These tests run the graph with a timeout. If the implementation does not drop the used
// sender, the test times out (FAIL). Passing = graph completes within limit = streams close.

/// Timeout for "graph must complete" tests. Short enough to fail fast when we hang.
const GRAPH_COMPLETION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// CodergenNode error path: no agent_cmd → node sends on error port.
/// Downstream (merge → exit) must see stream close; requires CodergenNode to drop err_tx after send.
/// CULPRIT when this times out: CodergenNode (src/nodes/codergen_node.rs) does not drop err_tx after send on error path. See history/CULPRIT-one-shot-sender-not-dropped.md.
#[tokio::test]
async fn tdd_codergen_error_path_graph_completes_within_timeout() {
  let dot = r#"
    digraph TddCodergenError {
      graph [goal="tdd codergen error path"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      fail [type=exec, command="false"]
      fix [label="Fix"]
      start -> fail
      fail -> fix [condition="outcome=fail"]
      fix -> exit [condition="outcome=fail"]
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let result = tokio::time::timeout(
    GRAPH_COMPLETION_TIMEOUT,
    streamweave_attractor::run_compiled_graph(
      &ast,
      streamweave_attractor::RunOptions {
        run_dir: None,
        agent_cmd: None,
        stage_dir: None,
      },
    ),
  )
  .await;
  assert!(
    result.is_ok(),
    "graph must complete within {:?} (codergen error path). \
     If this times out, the node that sent on the error port did not drop that sender.",
    GRAPH_COMPLETION_TIMEOUT
  );
  let run_result = result.unwrap().expect("run_compiled_graph");
  assert!(
    run_result.completed_nodes.contains(&"fix".to_string()),
    "fix (CodergenNode) should have run and completed; completed: {:?}",
    run_result.completed_nodes
  );
}

/// CodergenNode success path: agent_cmd that succeeds → node sends on out port.
/// Downstream must see stream close; requires CodergenNode to drop out_tx after send.
#[tokio::test]
async fn tdd_codergen_success_path_graph_completes_within_timeout() {
  let dot = r#"
    digraph TddCodergenSuccess {
      graph [goal="tdd codergen success path"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      fix [label="Fix"]
      start -> fix
      fix -> exit
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let result = tokio::time::timeout(
    GRAPH_COMPLETION_TIMEOUT,
    streamweave_attractor::run_compiled_graph(
      &ast,
      streamweave_attractor::RunOptions {
        run_dir: None,
        agent_cmd: Some("true".to_string()),
        stage_dir: None,
      },
    ),
  )
  .await;
  assert!(
    result.is_ok(),
    "graph must complete within {:?} (codergen success path). \
     If this times out, the node that sent on the out port did not drop that sender.",
    GRAPH_COMPLETION_TIMEOUT
  );
  let run_result = result.unwrap().expect("run_compiled_graph");
  assert!(
    run_result.completed_nodes.contains(&"fix".to_string()),
    "fix (CodergenNode) should have run and completed; completed: {:?}",
    run_result.completed_nodes
  );
}

/// ExecNode error path: command fails → node sends on error port.
/// Same assumption: sender for the used port must be dropped so graph completes.
#[tokio::test]
async fn tdd_exec_error_path_graph_completes_within_timeout() {
  let dot = r#"
    digraph TddExecError {
      graph [goal="tdd exec error path"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      fail [type=exec, command="false"]
      start -> fail
      fail -> exit [condition="outcome=success"]
      fail -> exit [condition="outcome=fail"]
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let result = tokio::time::timeout(
    GRAPH_COMPLETION_TIMEOUT,
    streamweave_attractor::run_compiled_graph(
      &ast,
      streamweave_attractor::RunOptions {
        run_dir: None,
        agent_cmd: None,
        stage_dir: None,
      },
    ),
  )
  .await;
  assert!(
    result.is_ok(),
    "graph must complete within {:?} (exec error path). \
     If this times out, ExecNode did not drop the error port sender after send.",
    GRAPH_COMPLETION_TIMEOUT
  );
}

/// ExecNode success path: command succeeds → node sends on out port.
#[tokio::test]
async fn tdd_exec_success_path_graph_completes_within_timeout() {
  let dot = r#"
    digraph TddExecSuccess {
      graph [goal="tdd exec success path"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      ok [type=exec, command="true"]
      start -> ok
      ok -> exit
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let result = tokio::time::timeout(
    GRAPH_COMPLETION_TIMEOUT,
    streamweave_attractor::run_compiled_graph(
      &ast,
      streamweave_attractor::RunOptions {
        run_dir: None,
        agent_cmd: None,
        stage_dir: None,
      },
    ),
  )
  .await;
  assert!(
    result.is_ok(),
    "graph must complete within {:?} (exec success path). \
     If this times out, ExecNode did not drop the out port sender after send.",
    GRAPH_COMPLETION_TIMEOUT
  );
}
