//! # streamweave-attractor
//!
//! Attractor pipeline implementation as a graph of StreamWeave nodes.
//! Implements [StrongDM's Attractor spec](https://github.com/strongdm/attractor).
//!
//! ## Architecture
//!
//! All pipeline logic is implemented as StreamWeave nodes:
//!
//! Pipeline logic is implemented as StreamWeave nodes (see `nodes` module).
//! Supporting nodes: ApplyContextUpdatesNode, CheckGoalGatesNode,
//! CreateCheckpointNode, FindStartNode, etc.

pub(crate) mod agent_run;
pub mod checkpoint_io;
pub mod compiler;
#[cfg(test)]
mod compiler_test;
pub mod dot_parser;
#[cfg(test)]
mod dot_parser_test;
pub mod nodes;
pub mod runner;
#[cfg(test)]
mod runner_test;
pub mod types;

pub use compiler::compile_attractor_graph;
pub use nodes::AttractorResult;
pub use runner::{run_compiled_graph, run_streamweave_graph};
pub use types::{AttractorGraph, AttractorNode, ExecutionState, NodeOutcome};
