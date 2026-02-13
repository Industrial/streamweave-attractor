//! Payload type that flows through the compiled StreamWeave graph.
//! Carries RunContext and the latest NodeOutcome so context flows start→nodes→exit
//! and context_updates from outcomes are applied along the path.

use super::{NodeOutcome, RunContext};

/// Payload flowing through the compiled graph: context plus optional last outcome.
/// Nodes that produce outcomes (Exec, Codergen) merge outcome.context_updates into context.
#[derive(Debug, Clone)]
pub struct GraphPayload {
  pub context: RunContext,
  pub outcome: Option<NodeOutcome>,
}

impl GraphPayload {
  pub fn new(context: RunContext, outcome: Option<NodeOutcome>) -> Self {
    Self { context, outcome }
  }

  /// Initial payload for graph entry (e.g. from start node).
  pub fn initial(context: RunContext) -> Self {
    Self {
      context,
      outcome: None,
    }
  }
}
