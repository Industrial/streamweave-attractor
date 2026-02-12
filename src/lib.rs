//! # streamweave-attractor
//!
//! Attractor pipeline implementation as a graph of StreamWeave nodes.
//! Implements [StrongDM's Attractor spec](https://github.com/strongdm/attractor).
//!
//! ## Architecture
//!
//! All pipeline logic is implemented as StreamWeave nodes:
//!
//! - **ParseDotNode**: Parse DOT source → AttractorGraph
//! - **ValidateGraphNode**: Validate graph (start/exit nodes)
//! - **InitContextNode**: Initialize ExecutionState
//! - **AttractorExecutionLoopNode**: Run traversal loop until terminal
//!
//! Supporting nodes (used internally or for composition):
//! - ExecuteHandlerNode, SelectEdgeNode, ApplyContextUpdatesNode,
//!   CheckGoalGatesNode, CreateCheckpointNode, FindStartNode

pub mod compiler;
#[cfg(test)]
mod compiler_test;
pub mod dot_parser;
#[cfg(test)]
mod dot_parser_test;
pub mod graph;
#[cfg(test)]
mod graph_test;
pub mod nodes;
pub mod types;

pub use compiler::compile_attractor_graph;
pub use graph::attractor_graph;
pub use nodes::{AttractorExecutionLoopNode, AttractorResult};
pub use types::{AttractorGraph, AttractorNode, ExecutionState, NodeOutcome};
