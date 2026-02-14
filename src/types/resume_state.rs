//! Resume state for --resume from execution log (execution.log.json only; no checkpoint.json).

use super::RunContext;
use serde::{Deserialize, Serialize};

/// Resume state (context, current node, completed nodes). Used by --resume from execution log only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeState {
  pub context: RunContext,
  pub current_node_id: String,
  pub completed_nodes: Vec<String>,
}
