//! Compile AttractorGraph (AST) to StreamWeave graph.
//!
//! Phase 1: Trivial start→exit with identity nodes.
//! Phase 2: ExecNode for exec, CodergenNode for codergen, identity for start/exit.
//! Phase 3: Conditional routing (outcome=success / outcome=fail) via OutcomeRouterNode.

use crate::nodes::{CodergenNode, ExecNode, IdentityNode, OutcomeRouterNode, validate_graph};
use crate::types::AttractorGraph;
use streamweave::graph_builder::GraphBuilder;
use streamweave::node::Node;
use tracing::{info, instrument};

/// Returns true if the condition string matches `outcome=success`.
#[instrument(level = "trace")]
fn condition_is_outcome_success(cond: Option<&str>) -> bool {
  cond
    .map(|c| {
      let c = c.trim().to_lowercase();
      c == "outcome=success" || c.starts_with("outcome=success")
    })
    .unwrap_or(false)
}

/// Returns true if the condition string matches `outcome=fail`.
#[instrument(level = "trace")]
fn condition_is_outcome_fail(cond: Option<&str>) -> bool {
  cond
    .map(|c| {
      let c = c.trim().to_lowercase();
      c == "outcome=fail" || c.starts_with("outcome=fail")
    })
    .unwrap_or(false)
}

/// Compiles an AttractorGraph (AST) to a StreamWeave Graph.
///
/// - Start/exit: IdentityNode (pass-through)
/// - Exec nodes: ExecNode with command (rejects exec without command per design §2.2)
/// - Codergen/other: CodergenNode (invokes ATTRACTOR_AGENT_CMD with prompt)
#[instrument(level = "trace", skip(ast))]
pub fn compile_attractor_graph(ast: &AttractorGraph) -> Result<streamweave::graph::Graph, String> {
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

  let mut routed_sources: std::collections::HashSet<String> = std::collections::HashSet::new();
  for e in &ast.edges {
    let from = &e.from_node;
    if routed_sources.contains(from) {
      continue;
    }
    let out_edges = ast.outgoing_edges(from);
    if out_edges.len() == 2 {
      let success_edge = out_edges
        .iter()
        .find(|e| condition_is_outcome_success(e.condition.as_deref()));
      let fail_edge = out_edges
        .iter()
        .find(|e| condition_is_outcome_fail(e.condition.as_deref()));
      if let (Some(se), Some(fe)) = (success_edge, fail_edge) {
        let router_id = format!("{}_router", from);
        builder = builder.add_node(
          &router_id,
          Box::new(OutcomeRouterNode::new(&router_id)) as Box<dyn Node>,
        );
        builder = builder.connect(from, "out", &router_id, "in");
        builder = builder.connect(&router_id, "success", &se.to_node, "in");
        builder = builder.connect(&router_id, "fail", &fe.to_node, "in");
        routed_sources.insert(from.clone());
        continue;
      }
    }
    builder = builder.connect(&e.from_node, "out", &e.to_node, "in");
  }

  let graph = builder
    .input::<std::sync::Arc<dyn std::any::Any + Send + Sync>>("input", &start_id, "in", None)
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
