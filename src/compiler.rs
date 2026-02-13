//! Compile AttractorGraph (AST) to StreamWeave graph.
//!
//! Phase 1: Trivial start→exit with identity nodes.
//! Phase 2: ExecNode for exec, CodergenNode for codergen, identity for start/exit.
//! Phase 3: Direct port routing: success -> "out", error -> "error" (no router).

use crate::nodes::{CodergenNode, ExecNode, IdentityNode, validate_graph};
use crate::types::AttractorGraph;
use streamweave::graph_builder::GraphBuilder;
use streamweave::node::Node;
use tracing::{info, instrument};

/// Returns true if the condition string matches `outcome=fail` or `outcome=error`.
#[instrument(level = "trace")]
fn condition_is_outcome_error(cond: Option<&str>) -> bool {
  cond
    .map(|c| {
      let c = c.trim().to_lowercase();
      c == "outcome=fail" || c.starts_with("outcome=fail")
      || c == "outcome=error" || c.starts_with("outcome=error")
    })
    .unwrap_or(false)
}

/// Compiles an AttractorGraph (AST) to a StreamWeave Graph.
///
/// - Start/exit: IdentityNode (pass-through)
/// - Exec nodes: ExecNode with command (rejects exec without command per design §2.2)
/// - Codergen/other: CodergenNode (invokes ATTRACTOR_AGENT_CMD with prompt)
///
/// When `entry_node_id` is `Some(id)`, the graph input is connected to that node (for resume).
/// When `None`, the graph input is connected to the start node.
#[instrument(level = "trace", skip(ast))]
pub fn compile_attractor_graph(
  ast: &AttractorGraph,
  entry_node_id: Option<&str>,
) -> Result<streamweave::graph::Graph, String> {
  info!("compiling AttractorGraph to StreamWeave graph");
  validate_graph::validate(ast)?;

  // DSL rule §2.2: reject exec nodes without command
  for (id, n) in &ast.nodes {
    if n.handler_type.as_deref() == Some("exec") && n.command.is_none() {
      return Err(format!("exec node '{}' requires a command attribute", id));
    }
  }

  let start_id = ast
    .find_start()
    .map(|n| n.id.clone())
    .ok_or("missing start node")?;
  let entry_id = match entry_node_id {
    Some(id) => {
      if !ast.nodes.contains_key(id) {
        return Err(format!("entry node '{}' is not a node in the graph", id));
      }
      id.to_string()
    }
    None => start_id.clone(),
  };
  let exit_id = ast
    .find_exit()
    .map(|n| n.id.clone())
    .ok_or("missing exit node")?;

  // StreamWeave's dataflow execution supports cycles (one channel per edge, one task per node).
  // Include all edges including fix→exec back-edges so fix-and-retry loops run in-graph.

  let mut builder = GraphBuilder::new("compiled_attractor");

  for (node_id, node) in &ast.nodes {
    let sw_node: Box<dyn Node> = match node.handler_type.as_deref().unwrap_or("codergen") {
      "start" | "exit" => Box::new(IdentityNode::new(&node.id)),
      "exec" => {
        let cmd = node.command.as_ref().expect("validated above").clone();
        Box::new(ExecNode::new(&node.id, cmd))
      }
      _ => {
        let prompt = node.prompt.as_deref().unwrap_or("").to_string();
        Box::new(CodergenNode::new(&node.id, prompt))
      }
    };
    builder = builder.add_node(node_id, sw_node);
  }

  for e in &ast.edges {
    let (source_port, target_node, target_port) = if condition_is_outcome_error(e.condition.as_deref()) {
      ("error", e.to_node.as_str(), "in")
    } else {
      // success or unconditional: use "out"
      ("out", e.to_node.as_str(), "in")
    };
    builder = builder.connect(&e.from_node, source_port, target_node, target_port);
  }

  let graph = builder
    .input::<std::sync::Arc<dyn std::any::Any + Send + Sync>>("input", &entry_id, "in", None)
    .output("output", &exit_id, "out")
    .build()
    .map_err(|e| e.to_string())?;

  info!(
    node_count = ast.nodes.len(),
    edge_count = ast.edges.len(),
    "compilation complete"
  );
  Ok(graph)
}
