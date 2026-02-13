//! Checkpoint for resumable execution.

use super::RunContext;
use serde::{Deserialize, Serialize};

/// Checkpoint for resumable execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
  pub context: RunContext,
  pub current_node_id: String,
  pub completed_nodes: Vec<String>,
}
