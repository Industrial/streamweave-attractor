//! Payload type that flows through the compiled StreamWeave graph.
//! Carries RunContext and the latest NodeOutcome so context flows start→nodes→exit
//! and context_updates from outcomes are applied along the path.
//! Tracks current_node_id and completed_nodes for checkpoint.

use super::{NodeOutcome, RunContext};
use tracing::instrument;

/// Payload flowing through the compiled graph: context plus optional last outcome.
/// Nodes that produce outcomes (Exec, Codergen) merge outcome.context_updates into context.
#[derive(Debug, Clone)]
pub struct GraphPayload {
  pub context: RunContext,
  pub outcome: Option<NodeOutcome>,
  /// Node id that last produced this payload (for checkpoint).
  pub current_node_id: String,
  /// Ordered list of node ids that have completed (for checkpoint).
  pub completed_nodes: Vec<String>,
}

impl GraphPayload {
  #[instrument(
    level = "trace",
    skip(context, outcome, current_node_id, completed_nodes)
  )]
  pub fn new(
    context: RunContext,
    outcome: Option<NodeOutcome>,
    current_node_id: String,
    completed_nodes: Vec<String>,
  ) -> Self {
    Self {
      context,
      outcome,
      current_node_id,
      completed_nodes,
    }
  }

  /// Initial payload for graph entry (e.g. from start node).
  #[instrument(level = "trace", skip(context, start_node_id))]
  pub fn initial(context: RunContext, start_node_id: impl Into<String>) -> Self {
    let start = start_node_id.into();
    Self {
      context,
      outcome: None,
      current_node_id: start.clone(),
      completed_nodes: vec![],
    }
  }

  /// Initial payload for resume: context, current node, and completed nodes from execution log state.
  pub fn from_resume_state(st: &super::ResumeState) -> Self {
    Self {
      context: st.context.clone(),
      outcome: None,
      current_node_id: st.current_node_id.clone(),
      completed_nodes: st.completed_nodes.clone(),
    }
  }

  /// Initial payload from checkpoint (in-graph CreateCheckpointNode output).
  pub fn from_checkpoint(cp: &super::Checkpoint) -> Self {
    Self::from_resume_state(cp)
  }

  /// Returns a new payload with this node recorded as current and completed (for nodes that emit).
  #[instrument(level = "trace", skip(self, node_id))]
  pub fn with_node_completed(&self, node_id: impl Into<String>) -> Self {
    let node_id = node_id.into();
    let mut completed = self.completed_nodes.clone();
    completed.push(node_id.clone());
    Self {
      context: self.context.clone(),
      outcome: self.outcome.clone(),
      current_node_id: node_id,
      completed_nodes: completed,
    }
  }
}
