//! Build the Attractor pipeline as a StreamWeave graph.

use crate::nodes::ParseDotNode;
use crate::nodes::execution_loop::AttractorExecutionLoopNode;
use crate::nodes::init_context::InitContextNode;
use crate::nodes::validate_graph::ValidateGraphNode;
use streamweave::graph;
use tracing::{info, instrument};

/// Build the full Attractor pipeline graph.
///
/// Pipeline: ParseDot → Validate → Init → ExecutionLoop
#[instrument(level = "trace")]
pub fn attractor_graph() -> Result<streamweave::graph::Graph, String> {
  info!("building attractor pipeline graph");
  Ok(graph! {
    parse: ParseDotNode::new("parse"),
    validate: ValidateGraphNode::new("validate"),
    init: InitContextNode::new("init"),
    execute: AttractorExecutionLoopNode::new("execute"),
    graph.input => parse.in,
    parse.out => validate.in,
    validate.out => init.in,
    init.out => execute.in,
    execute.out => graph.output,
    execute.error => graph.error
  })
}
