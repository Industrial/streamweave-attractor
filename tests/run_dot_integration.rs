//! Integration tests for the run_dot CLI and compiled graph path.
//!
//! - CLI: runs the binary via `cargo run --bin run_dot` with temp .dot files.
//! - Compiled path: pre-push.dot via run_compiled_graph (library).

use std::process::Command;

fn run_run_dot(args: &[&str]) -> std::process::Output {
  run_run_dot_with_env(args, &[])
}

/// Run run_dot with optional env vars. Each pair is (key, value); pass empty to inherit.
fn run_run_dot_with_env(args: &[&str], env_add: &[(&str, &str)]) -> std::process::Output {
  let mut cmd = Command::new("cargo");
  cmd.args(["run", "--bin", "run_dot", "--"]).args(args);
  for (k, v) in env_add {
    cmd.env(k, v);
  }
  cmd.output().expect("run cargo run --bin run_dot")
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

#[test]
fn run_dot_execution_log_cli_writes_log_file() {
  let dir = tempfile::tempdir().expect("temp dir");
  let path = dir.path().join("minimal.dot");
  let log_path = dir.path().join("execution.log.json");
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
  let log_path_str = log_path.to_str().expect("log path");
  let out = run_run_dot(&["--execution-log", log_path_str, path_str]);
  assert!(
    out.status.success(),
    "stderr: {} stdout: {}",
    String::from_utf8_lossy(&out.stderr),
    String::from_utf8_lossy(&out.stdout)
  );
  assert!(log_path.exists(), "execution log file should exist");
  let content = std::fs::read_to_string(&log_path).expect("read execution log");
  let log: serde_json::Value = serde_json::from_str(&content).expect("parse execution log JSON");
  assert_eq!(log["version"], 1);
  assert_eq!(log["goal"], "test");
  assert_eq!(log["final_status"], "success");
}

#[test]
fn run_dot_execution_log_cli_default_path_under_stage_dir() {
  let dir = tempfile::tempdir().expect("temp dir");
  let stage = dir.path().join("stage");
  std::fs::create_dir_all(&stage).expect("create stage dir");
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
  let stage_str = stage.to_str().expect("stage");
  let out = run_run_dot(&["--execution-log", "--stage-dir", stage_str, path_str]);
  assert!(
    out.status.success(),
    "stderr: {} stdout: {}",
    String::from_utf8_lossy(&out.stderr),
    String::from_utf8_lossy(&out.stdout)
  );
  let default_log = stage.join("execution.log.json");
  assert!(
    default_log.exists(),
    "execution log should be at <stage_dir>/execution.log.json"
  );
  let content = std::fs::read_to_string(&default_log).expect("read execution log");
  let log: serde_json::Value = serde_json::from_str(&content).expect("parse execution log JSON");
  assert_eq!(log["final_status"], "success");
}

#[test]
fn run_dot_execution_log_env_1_uses_default_path() {
  let dir = tempfile::tempdir().expect("temp dir");
  let stage = dir.path().join("stage");
  std::fs::create_dir_all(&stage).expect("create stage dir");
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
  let stage_str = stage.to_str().expect("stage");
  let out = run_run_dot_with_env(
    &["--stage-dir", stage_str, path_str],
    &[("ATTRACTOR_EXECUTION_LOG", "1")],
  );
  assert!(
    out.status.success(),
    "stderr: {} stdout: {}",
    String::from_utf8_lossy(&out.stderr),
    String::from_utf8_lossy(&out.stdout)
  );
  let default_log = stage.join("execution.log.json");
  assert!(
    default_log.exists(),
    "ATTRACTOR_EXECUTION_LOG=1 should write to <stage_dir>/execution.log.json"
  );
  let content = std::fs::read_to_string(&default_log).expect("read execution log");
  let log: serde_json::Value = serde_json::from_str(&content).expect("parse execution log JSON");
  assert_eq!(log["final_status"], "success");
}

#[test]
fn run_dot_execution_log_env_true_uses_default_path() {
  let dir = tempfile::tempdir().expect("temp dir");
  let stage = dir.path().join("stage");
  std::fs::create_dir_all(&stage).expect("create stage dir");
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
  let stage_str = stage.to_str().expect("stage");
  let out = run_run_dot_with_env(
    &["--stage-dir", stage_str, path_str],
    &[("ATTRACTOR_EXECUTION_LOG", "true")],
  );
  assert!(
    out.status.success(),
    "stderr: {} stdout: {}",
    String::from_utf8_lossy(&out.stderr),
    String::from_utf8_lossy(&out.stdout)
  );
  let default_log = stage.join("execution.log.json");
  assert!(
    default_log.exists(),
    "ATTRACTOR_EXECUTION_LOG=true should write to <stage_dir>/execution.log.json"
  );
}

#[test]
fn run_dot_execution_log_env_path_uses_that_path() {
  let dir = tempfile::tempdir().expect("temp dir");
  let path = dir.path().join("minimal.dot");
  let log_path = dir.path().join("env_log.json");
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
  let log_path_str = log_path.to_str().expect("log path");
  let out = run_run_dot_with_env(&[path_str], &[("ATTRACTOR_EXECUTION_LOG", log_path_str)]);
  assert!(
    out.status.success(),
    "stderr: {} stdout: {}",
    String::from_utf8_lossy(&out.stderr),
    String::from_utf8_lossy(&out.stdout)
  );
  assert!(
    log_path.exists(),
    "ATTRACTOR_EXECUTION_LOG=<path> should write to that path"
  );
  let content = std::fs::read_to_string(&log_path).expect("read execution log");
  let log: serde_json::Value = serde_json::from_str(&content).expect("parse execution log JSON");
  assert_eq!(log["final_status"], "success");
}

#[test]
fn run_dot_execution_log_cli_overrides_env() {
  let dir = tempfile::tempdir().expect("temp dir");
  let path = dir.path().join("minimal.dot");
  let cli_log = dir.path().join("cli_log.json");
  let env_log = dir.path().join("env_log.json");
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
  let cli_log_str = cli_log.to_str().expect("cli log path");
  let env_log_str = env_log.to_str().expect("env log path");
  let out = run_run_dot_with_env(
    &["--execution-log", cli_log_str, path_str],
    &[("ATTRACTOR_EXECUTION_LOG", env_log_str)],
  );
  assert!(
    out.status.success(),
    "stderr: {} stdout: {}",
    String::from_utf8_lossy(&out.stderr),
    String::from_utf8_lossy(&out.stdout)
  );
  assert!(cli_log.exists(), "CLI --execution-log path should be used");
  assert!(
    !env_log.exists(),
    "env path should be ignored when --execution-log is set"
  );
}

/// When execution_log_path is set, runner writes execution.log.json on completion (success path).
#[tokio::test]
async fn execution_log_path_writes_execution_log_json() {
  let dir = tempfile::tempdir().expect("temp dir");
  let log_path = dir.path().join("execution.log.json");
  let dot = r#"
    digraph G {
      graph [goal="exec log test"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let _ = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: None,
      resume_checkpoint: None,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: Some(log_path.clone()),
    },
  )
  .await
  .expect("run_compiled_graph");
  let content = std::fs::read_to_string(&log_path).expect("read execution log");
  let log: serde_json::Value = serde_json::from_str(&content).expect("parse execution log JSON");
  assert_eq!(log["version"], 1);
  assert_eq!(log["goal"], "exec log test");
  assert_eq!(log["final_status"], "success");
  assert!(log["steps"].as_array().is_some());
  let steps = log["steps"].as_array().unwrap();
  assert!(!steps.is_empty(), "expected at least one step");
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
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
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
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
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
      resume_checkpoint: None,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
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

/// Run pipeline with run_dir, then resume from that checkpoint and assert completion.
#[tokio::test]
async fn resume_from_checkpoint_completes() {
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

  // First run: write checkpoint
  streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: Some(run_dir.path()),
      resume_checkpoint: None,
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
    },
  )
  .await
  .expect("first run");

  let cp_path = run_dir.path().join("checkpoint.json");
  let cp =
    streamweave_attractor::checkpoint_io::load_checkpoint(&cp_path).expect("load checkpoint");

  // Resume run: same graph, from checkpoint
  let result = streamweave_attractor::run_compiled_graph(
    &ast,
    streamweave_attractor::RunOptions {
      run_dir: Some(run_dir.path()),
      resume_checkpoint: Some(cp),
      agent_cmd: None,
      stage_dir: None,
      execution_log_path: None,
    },
  )
  .await
  .expect("resume run");

  assert!(
    result.completed_nodes.contains(&"exit".to_string()),
    "resume should complete through exit; completed: {:?}",
    result.completed_nodes
  );
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
        resume_checkpoint: None,
        agent_cmd: None,
        stage_dir: None,
        execution_log_path: None,
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
        resume_checkpoint: None,
        agent_cmd: Some("true".to_string()),
        stage_dir: None,
        execution_log_path: None,
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
        resume_checkpoint: None,
        agent_cmd: None,
        stage_dir: None,
        execution_log_path: None,
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

/// Cyclic graph with Merge: start and loop_back feed merge → check_ready (exec) → middle → loop_back.
/// When check_ready fails, it sends on error port and must break to avoid deadlock with MergeNode
/// (Merge waits for both inputs to close; loop_back never closes if we hang).
#[tokio::test]
async fn tdd_cyclic_exec_error_path_graph_completes_within_timeout() {
  let dot = r#"
    digraph TddCyclicExecError {
      graph [goal="tdd cyclic exec error path"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      check [type=exec, command="false", label="Check"]
      middle [type=exec, command="true", label="Middle"]
      loop_back [type=exec, command="true", label="LoopBack"]
      start -> check
      loop_back -> check
      check -> exit [condition="outcome=fail"]
      check -> middle [condition="outcome=success"]
      middle -> loop_back
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let result = tokio::time::timeout(
    GRAPH_COMPLETION_TIMEOUT,
    streamweave_attractor::run_compiled_graph(
      &ast,
      streamweave_attractor::RunOptions {
        run_dir: None,
        resume_checkpoint: None,
        agent_cmd: None,
        stage_dir: None,
        execution_log_path: None,
      },
    ),
  )
  .await;
  assert!(
    result.is_ok(),
    "cyclic graph must complete within {:?} (exec error path). \
     If this times out, ExecNode did not break after error send, causing MergeNode deadlock.",
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
        resume_checkpoint: None,
        agent_cmd: None,
        stage_dir: None,
        execution_log_path: None,
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

/// Cyclic graph with Merge: start and loop_back feed merge → check (CodergenNode) → middle → loop_back.
/// When check fails (no agent_cmd), it sends on error port and must break to avoid deadlock.
#[tokio::test]
async fn tdd_cyclic_codergen_error_path_graph_completes_within_timeout() {
  let dot = r#"
    digraph TddCyclicCodergenError {
      graph [goal="tdd cyclic codergen error path"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      check [label="Check"]
      middle [type=exec, command="true", label="Middle"]
      loop_back [type=exec, command="true", label="LoopBack"]
      start -> check
      loop_back -> check
      check -> exit [condition="outcome=fail"]
      check -> middle [condition="outcome=success"]
      middle -> loop_back
    }
  "#;
  let ast = streamweave_attractor::dot_parser::parse_dot(dot).expect("parse dot");
  let result = tokio::time::timeout(
    GRAPH_COMPLETION_TIMEOUT,
    streamweave_attractor::run_compiled_graph(
      &ast,
      streamweave_attractor::RunOptions {
        run_dir: None,
        resume_checkpoint: None,
        agent_cmd: None,
        stage_dir: None,
        execution_log_path: None,
      },
    ),
  )
  .await;
  assert!(
    result.is_ok(),
    "cyclic graph must complete within {:?} (codergen error path).      If this times out, CodergenNode did not break after error send.",
    GRAPH_COMPLETION_TIMEOUT
  );
}
