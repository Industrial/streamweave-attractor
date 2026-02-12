//! Compile AttractorGraph (AST) to StreamWeave graph.
//!
//! Phase 1: Trivial start→exit with identity nodes.
//! Phase 2: ExecNode for exec handlers, identity for start/exit, stub for codergen.
//! Phase 3: Conditional routing (outcome=success / outcome=fail) via OutcomeRouterNode.

use crate::nodes::{ExecNode, FixNode, IdentityNode, OutcomeRouterNode, validate_graph};
use crate::types::AttractorGraph;
use streamweave::graph_builder::GraphBuilder;
use streamweave::node::Node;

fn condition_is_outcome_success(cond: Option<&str>) -> bool {
  cond
    .map(|c| {
      let c = c.trim().to_lowercase();
      c == "outcome=success" || c.starts_with("outcome=success")
    })
    .unwrap_or(false)
}

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
/// - Codergen/other: IdentityNode stub
pub fn compile_attractor_graph(ast: &AttractorGraph) -> Result<streamweave::graph::Graph, String> {
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

  let fix_node_ids: std::collections::HashSet<String> = ast
    .edges
    .iter()
    .filter(|e| condition_is_outcome_fail(e.condition.as_deref()))
    .map(|e| e.to_node.clone())
    .collect();

  let mut builder = GraphBuilder::new("compiled_attractor");

  for (node_id, node) in &ast.nodes {
    let sw_node: Box<dyn Node> = match node.handler_type.as_deref().unwrap_or("codergen") {
      "start" | "exit" => Box::new(IdentityNode::new(&node.id)),
      "exec" => {
        let cmd = node.command.as_ref().expect("validated above").clone();
        Box::new(ExecNode::new(&node.id, cmd))
      }
      _ => {
        if fix_node_ids.contains(node_id) {
          Box::new(FixNode::new(&node.id))
        } else {
          Box::new(IdentityNode::new(&node.id)) // codergen stub
        }
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
      if success_edge.is_some() && fail_edge.is_some() {
        let router_id = format!("{}_router", from);
        builder = builder.add_node(
          &router_id,
          Box::new(OutcomeRouterNode::new(&router_id)) as Box<dyn Node>,
        );
        builder = builder.connect(from, "out", &router_id, "in");
        builder = builder.connect(&router_id, "success", &success_edge.unwrap().to_node, "in");
        builder = builder.connect(&router_id, "fail", &fail_edge.unwrap().to_node, "in");
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

  Ok(graph)
}
