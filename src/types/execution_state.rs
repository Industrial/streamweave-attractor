//! Execution state for one step of the Attractor loop.

use std::collections::HashMap;

use super::{AttractorGraph, NodeOutcome, RunContext};

/// Execution state for one step of the Attractor loop.
#[derive(Debug, Clone)]
pub struct ExecutionState {
  pub graph: AttractorGraph,
  pub context: RunContext,
  pub current_node_id: String,
  pub completed_nodes: Vec<String>,
  pub node_outcomes: HashMap<String, NodeOutcome>,
}

impl ExecutionState {
  pub fn is_done(&self) -> bool {
    self
      .graph
      .nodes
      .get(&self.current_node_id)
      .map(|n| n.is_terminal())
      .unwrap_or(false)
  }
}
