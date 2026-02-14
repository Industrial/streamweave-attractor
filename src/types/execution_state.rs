//! Execution state for one step of the Attractor loop.

use std::collections::HashMap;
use tracing::instrument;

use super::{AttractorGraph, ExecutionStepEntry, NodeOutcome, RunContext};

/// Execution state for one step of the Attractor loop.
#[derive(Debug, Clone)]
pub struct ExecutionState {
  pub graph: AttractorGraph,
  pub context: RunContext,
  pub current_node_id: String,
  pub completed_nodes: Vec<String>,
  pub node_outcomes: HashMap<String, NodeOutcome>,
  /// Optional step log sink; when `Some`, each node execution + select_edge pushes one entry.
  pub step_log: Option<Vec<ExecutionStepEntry>>,
}

impl ExecutionState {
  #[instrument(level = "trace")]
  pub fn is_done(&self) -> bool {
    self
      .graph
      .nodes
      .get(&self.current_node_id)
      .map(|n| n.is_terminal())
      .unwrap_or(false)
  }
}
