//! DTOs for execution.log.json: log of graph execution steps for debugging.
//!
//! Maps from [RunContext](super::RunContext) and [NodeOutcome](super::NodeOutcome) produced
//! during the run.

use serde::{Deserialize, Serialize};

use super::{NodeOutcome, RunContext};

/// One recorded step in the execution log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStepEntry {
  /// 1-based step index.
  pub step: u32,
  /// Node that was executed.
  pub node_id: String,
  /// Handler type (e.g. "start", "exit", "codergen", "exec").
  pub handler_type: Option<String>,
  /// Context before executing the node.
  pub context_before: RunContext,
  /// Outcome of the node execution.
  pub outcome: NodeOutcome,
  /// Context after applying outcome context_updates.
  pub context_after: RunContext,
  /// Next node selected by the edge (if any).
  pub next_node_id: Option<String>,
  /// completed_nodes list after this step.
  pub completed_nodes_after: Vec<String>,
}

impl ExecutionStepEntry {
  /// Build a step entry from RunContext and NodeOutcome (before/after context and outcome).
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    step: u32,
    node_id: impl Into<String>,
    handler_type: Option<String>,
    context_before: RunContext,
    outcome: NodeOutcome,
    context_after: RunContext,
    next_node_id: Option<String>,
    completed_nodes_after: Vec<String>,
  ) -> Self {
    Self {
      step,
      node_id: node_id.into(),
      handler_type,
      context_before,
      outcome,
      context_after,
      next_node_id,
      completed_nodes_after,
    }
  }
}

/// Root structure for execution.log.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
  /// Log format version.
  pub version: u32,
  /// Goal description (e.g. from graph or CLI).
  pub goal: String,
  /// ISO 8601 timestamp when the run started.
  pub started_at: String,
  /// ISO 8601 timestamp when the run finished (None if still running).
  pub finished_at: Option<String>,
  /// Final outcome status when the run ended (e.g. "success", "error").
  pub final_status: String,
  /// Node IDs completed when the run ended.
  pub completed_nodes: Vec<String>,
  /// Recorded steps in order.
  pub steps: Vec<ExecutionStepEntry>,
}

#[cfg(test)]
mod tests {
  use super::{ExecutionLog, ExecutionStepEntry};
  use crate::types::{NodeOutcome, RunContext};
  use std::collections::HashMap;

  #[test]
  fn execution_step_entry_serializes_to_json() {
    let mut ctx_before: RunContext = HashMap::new();
    ctx_before.insert("goal".to_string(), "test goal".to_string());
    let mut ctx_after = ctx_before.clone();
    ctx_after.insert("step".to_string(), "1".to_string());
    let outcome = NodeOutcome::success("ok");
    let entry = ExecutionStepEntry::new(
      1,
      "start",
      Some("start".to_string()),
      ctx_before,
      outcome,
      ctx_after,
      Some("next".to_string()),
      vec!["start".to_string()],
    );
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["step"], 1);
    assert_eq!(parsed["node_id"], "start");
    assert_eq!(parsed["handler_type"], "start");
    assert_eq!(parsed["next_node_id"], "next");
    assert_eq!(
      parsed["completed_nodes_after"],
      serde_json::json!(["start"])
    );
    assert_eq!(parsed["outcome"]["status"], "success");
  }

  #[test]
  fn execution_log_serializes_to_json() {
    let mut ctx: RunContext = HashMap::new();
    ctx.insert("goal".to_string(), "run".to_string());
    let outcome = NodeOutcome::success("done");
    let step = ExecutionStepEntry::new(
      1,
      "n1",
      Some("exec".to_string()),
      HashMap::new(),
      outcome,
      ctx.clone(),
      Some("exit".to_string()),
      vec!["n1".to_string()],
    );
    let log = ExecutionLog {
      version: 1,
      goal: "run".to_string(),
      started_at: "2026-02-14T10:00:00Z".to_string(),
      finished_at: Some("2026-02-14T10:01:00Z".to_string()),
      final_status: "success".to_string(),
      completed_nodes: vec!["n1".to_string()],
      steps: vec![step],
    };
    let json = serde_json::to_string(&log).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["version"], 1);
    assert_eq!(parsed["goal"], "run");
    assert_eq!(parsed["started_at"], "2026-02-14T10:00:00Z");
    assert_eq!(parsed["finished_at"], "2026-02-14T10:01:00Z");
    assert_eq!(parsed["final_status"], "success");
    assert_eq!(parsed["completed_nodes"], serde_json::json!(["n1"]));
    assert_eq!(parsed["steps"].as_array().unwrap().len(), 1);
  }
}
