//! Load execution.log.json and derive resume state (for --resume when log is single source).

use crate::types::{Checkpoint, ExecutionLog};
use std::path::Path;

/// Default filename for execution log under a run directory.
pub const EXECUTION_LOG_FILENAME: &str = "execution.log.json";

/// Loads an execution log from `path`. Returns error if file is missing or invalid JSON.
pub fn load_execution_log(path: &Path) -> Result<ExecutionLog, std::io::Error> {
  let bytes = std::fs::read(path)?;
  serde_json::from_slice(&bytes)
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Writes a partial execution log to `path` (rewrite after each step).
/// Always writes with `finished_at: None` so the file represents an in-progress run.
/// Creates parent directory if needed.
pub fn write_execution_log_partial(path: &Path, log: &ExecutionLog) -> Result<(), std::io::Error> {
  let partial = ExecutionLog {
    finished_at: None,
    ..log.clone()
  };
  let json = serde_json::to_string_pretty(&partial)
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::write(path, json)
}

/// Resume state derived from an execution log.
pub struct ResumeFromLog {
  /// Checkpoint to pass to runner (context, current_node_id, completed_nodes).
  pub checkpoint: Checkpoint,
  /// True if the log indicates the run already completed (finished_at set).
  pub already_completed: bool,
}

/// Derives resume state from a loaded execution log.
/// - If `finished_at` is set: run completed; returns checkpoint with `current_node_id` set to `exit_node_id` (so runner returns already_completed) and `already_completed: true`.
/// - If partial (no `finished_at`): returns checkpoint from last step and `already_completed: false`.
/// - If log has no steps and no finished_at: returns None.
pub fn resume_state_from_log(
  log: &ExecutionLog,
  exit_node_id: Option<&str>,
) -> Option<ResumeFromLog> {
  let already_completed = log.finished_at.is_some();
  if already_completed {
    let current_node_id = exit_node_id
      .map(String::from)
      .or_else(|| log.completed_nodes.last().cloned())
      .unwrap_or_default();
    return Some(ResumeFromLog {
      checkpoint: Checkpoint {
        context: log
          .steps
          .last()
          .map(|s| s.context_after.clone())
          .unwrap_or_default(),
        current_node_id,
        completed_nodes: log.completed_nodes.clone(),
      },
      already_completed: true,
    });
  }
  let last = log.steps.last()?;
  let current_node_id = last
    .next_node_id
    .clone()
    .or_else(|| last.completed_nodes_after.last().cloned())
    .unwrap_or_default();
  Some(ResumeFromLog {
    checkpoint: Checkpoint {
      context: last.context_after.clone(),
      current_node_id,
      completed_nodes: last.completed_nodes_after.clone(),
    },
    already_completed: false,
  })
}

#[cfg(test)]
mod tests {
  use super::{
    load_execution_log, resume_state_from_log, write_execution_log_partial,
    EXECUTION_LOG_FILENAME,
  };
  use crate::types::{ExecutionLog, ExecutionStepEntry, NodeOutcome};
  use std::collections::HashMap;

  #[test]
  fn write_execution_log_partial_omits_finished_at() {
    let mut ctx: HashMap<String, String> = HashMap::new();
    ctx.insert("goal".to_string(), "test".to_string());
    let step = ExecutionStepEntry::new(
      1,
      "start",
      Some("start".to_string()),
      HashMap::new(),
      NodeOutcome::success("ok"),
      ctx.clone(),
      Some("exit".to_string()),
      vec!["start".to_string()],
    );
    let log = ExecutionLog {
      version: 1,
      goal: "partial".to_string(),
      started_at: "2026-02-14T10:00:00Z".to_string(),
      finished_at: Some("2026-02-14T10:01:00Z".to_string()), // must be stripped
      final_status: "success".to_string(),
      completed_nodes: vec!["start".to_string()],
      steps: vec![step],
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("execution.log.json");
    write_execution_log_partial(&path, &log).expect("write partial");
    let loaded = load_execution_log(&path).expect("load");
    assert_eq!(loaded.goal, "partial");
    assert_eq!(loaded.finished_at, None);
    assert_eq!(loaded.steps.len(), 1);
  }

  #[test]
  fn load_execution_log_roundtrip_and_resume_state() {
    let mut ctx: HashMap<String, String> = HashMap::new();
    ctx.insert("goal".to_string(), "test".to_string());
    let step = ExecutionStepEntry::new(
      1,
      "start",
      Some("start".to_string()),
      HashMap::new(),
      NodeOutcome::success("ok"),
      ctx.clone(),
      Some("exit".to_string()),
      vec!["start".to_string()],
    );
    let log = ExecutionLog {
      version: 1,
      goal: "test".to_string(),
      started_at: "2026-02-14T10:00:00Z".to_string(),
      finished_at: Some("2026-02-14T10:01:00Z".to_string()),
      final_status: "success".to_string(),
      completed_nodes: vec!["start".to_string(), "exit".to_string()],
      steps: vec![step],
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(EXECUTION_LOG_FILENAME);
    let json = serde_json::to_string_pretty(&log).unwrap();
    std::fs::write(&path, json).unwrap();
    let loaded = load_execution_log(&path).expect("load");
    assert_eq!(loaded.goal, "test");
    assert_eq!(loaded.completed_nodes, vec!["start", "exit"]);
    let r = resume_state_from_log(&loaded, Some("exit")).expect("resume state");
    assert!(r.already_completed);
    assert_eq!(r.checkpoint.current_node_id, "exit");
    assert_eq!(r.checkpoint.completed_nodes, vec!["start", "exit"]);
  }
}
