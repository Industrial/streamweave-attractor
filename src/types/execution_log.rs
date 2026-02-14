//! DTOs for execution.log.json: log of graph execution steps for debugging.
//!
//! Maps from [RunContext](super::RunContext) and [NodeOutcome](super::NodeOutcome) produced
//! during the run.

use serde::Serialize;

use super::{NodeOutcome, RunContext};

/// One recorded step in the execution log.
#[derive(Debug, Clone, Serialize)]
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

/// Root structure for execution.log.json.
#[derive(Debug, Clone, Serialize)]
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
