//! Attractor pipeline types per [attractor-spec](https://github.com/strongdm/attractor/blob/main/attractor-spec.md).
//!
//! These types flow through the StreamWeave graph as `Arc<dyn Any>`.

use std::collections::HashMap;

mod attractor_edge;
#[cfg(test)]
mod attractor_edge_test;
mod attractor_graph;
#[cfg(test)]
mod attractor_graph_test;
mod attractor_node;
#[cfg(test)]
mod attractor_node_test;
mod checkpoint;
#[cfg(test)]
mod checkpoint_test;
mod execution_log;
mod execution_state;
#[cfg(test)]
mod execution_state_test;
mod graph_payload;
#[cfg(test)]
mod graph_payload_test;
mod node_outcome;
#[cfg(test)]
mod node_outcome_test;
mod outcome_status;
#[cfg(test)]
mod outcome_status_test;

pub use attractor_edge::AttractorEdge;
pub use attractor_graph::AttractorGraph;
pub use attractor_node::AttractorNode;
pub use checkpoint::Checkpoint;
pub use execution_log::{ExecutionLog, ExecutionStepEntry};
pub use execution_state::ExecutionState;
pub use graph_payload::GraphPayload;
pub use node_outcome::NodeOutcome;
pub use outcome_status::OutcomeStatus;

/// Key-value context shared across the pipeline run.
pub type RunContext = HashMap<String, String>;
