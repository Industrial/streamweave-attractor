//! Parsed Attractor pipeline graph (DOT).

use std::collections::HashMap;
use tracing::instrument;

use super::{AttractorEdge, AttractorNode};

/// Parsed Attractor pipeline graph (DOT).
#[derive(Debug, Clone)]
pub struct AttractorGraph {
  pub goal: String,
  pub nodes: HashMap<String, AttractorNode>,
  pub edges: Vec<AttractorEdge>,
  pub default_max_retry: u32,
}

impl AttractorGraph {
  #[instrument(level = "trace")]
  pub fn find_start(&self) -> Option<&AttractorNode> {
    self.nodes.values().find(|n| n.is_start())
  }

  #[instrument(level = "trace")]
  pub fn find_exit(&self) -> Option<&AttractorNode> {
    self.nodes.values().find(|n| n.is_exit())
  }

  #[instrument(level = "trace")]
  pub fn outgoing_edges(&self, node_id: &str) -> Vec<&AttractorEdge> {
    self
      .edges
      .iter()
      .filter(|e| e.from_node == node_id)
      .collect()
  }
}
