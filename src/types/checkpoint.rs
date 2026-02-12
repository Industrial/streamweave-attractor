//! Checkpoint for resumable execution.

use super::RunContext;

/// Checkpoint for resumable execution.
#[derive(Debug, Clone)]
pub struct Checkpoint {
  pub context: RunContext,
  pub current_node_id: String,
  pub completed_nodes: Vec<String>,
}
