//! Build the Attractor pipeline as a StreamWeave graph.

use crate::nodes::{AttractorExecutionLoopNode, InitContextNode, ParseDotNode, ValidateGraphNode};
use streamweave::graph;

/// Build the full Attractor pipeline graph.
///
/// Pipeline: ParseDot → Validate → Init → ExecutionLoop
pub fn attractor_graph() -> Result<streamweave::graph::Graph, String> {
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
