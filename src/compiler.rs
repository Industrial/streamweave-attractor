//! Compile AttractorGraph (AST) to StreamWeave graph.
//!
//! Phase 1: Trivial start→exit with identity nodes.
//! Phase 2: ExecNode for exec, CodergenNode for codergen, identity for start/exit.
//! Phase 3: Direct port routing: success -> "out", error -> "error" (no router).
//! Phase 4: Multiple edges to same (node, port) are merged via StreamWeave MergeNode.

use crate::nodes::{CodergenNode, ExecNode, IdentityNode, validate_graph};
use crate::types::AttractorGraph;
use std::collections::HashMap;
use std::path::Path;
use streamweave::graph_builder::GraphBuilder;
use streamweave::node::Node;
use streamweave::nodes::stream::MergeNode;
use tracing::{info, instrument};

/// Returns true if the condition string matches `outcome=fail` or `outcome=error`.
#[instrument(level = "trace")]
fn condition_is_outcome_error(cond: Option<&str>) -> bool {
  cond
    .map(|c| {
      let c = c.trim().to_lowercase();
      c == "outcome=fail"
        || c.starts_with("outcome=fail")
        || c == "outcome=error"
        || c.starts_with("outcome=error")
    })
    .unwrap_or(false)
}

/// Compiles an AttractorGraph (AST) to a StreamWeave Graph.
///
/// - Start/exit: IdentityNode (pass-through)
/// - Exec nodes: ExecNode with command (rejects exec without command per design §2.2)
/// - Codergen/other: CodergenNode (invokes ATTRACTOR_AGENT_CMD with prompt)
///
/// When `entry_node_id` is `Some(id)`, the graph input is connected to that node;
/// when `None`, to the start node.
#[instrument(level = "trace", skip(ast))]
pub fn compile_attractor_graph(
  ast: &AttractorGraph,
  entry_node_id: Option<&str>,
  agent_cmd: Option<&str>,
  stage_dir: Option<&Path>,
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

  // Trivial graph (start -> exit only): use predefined graph! pipeline
  if ast.nodes.len() == 2
    && ast.edges.len() == 1
    && start_id == "start"
    && exit_id == "exit"
    && entry_id == start_id
  {
    let e = &ast.edges[0];
    if e.from_node == start_id && e.to_node == exit_id && e.condition.is_none() {
      return Ok(crate::graphs::trivial_start_exit_graph());
    }
  }

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
        let cmd = agent_cmd.map(String::from);
        let dir = stage_dir.map(std::path::PathBuf::from);
        Box::new(CodergenNode::new(&node.id, prompt, cmd, dir))
      }
    };
    builder = builder.add_node(node_id, sw_node);
  }

  let resolved: Vec<(String, String, String, String)> = ast
    .edges
    .iter()
    .map(|e| {
      let (source_port, target_port) = if condition_is_outcome_error(e.condition.as_deref()) {
        ("error".to_string(), "in".to_string())
      } else {
        ("out".to_string(), "in".to_string())
      };
      (
        e.from_node.clone(),
        source_port,
        e.to_node.clone(),
        target_port,
      )
    })
    .collect();
  let mut groups: HashMap<(String, String), Vec<(String, String)>> = HashMap::new();
  for (from, src_port, to, to_port) in &resolved {
    let key = (to.clone(), to_port.clone());
    groups
      .entry(key)
      .or_default()
      .push((from.clone(), src_port.clone()));
  }
  for ((to_node, to_port), list) in &groups {
    if list.len() > 1 {
      let merge_id = format!("merge_{}_{}", to_node, to_port);
      let merge = MergeNode::new(merge_id.clone(), list.len());
      builder = builder.add_node(&merge_id, Box::new(merge));
    }
  }
  for ((to_node, to_port), list) in &groups {
    if list.len() == 1 {
      let (from, src_port) = &list[0];
      builder = builder.connect(from, src_port, to_node, to_port);
    } else {
      let merge_id = format!("merge_{}_{}", to_node, to_port);
      for (i, (from, src_port)) in list.iter().enumerate() {
        builder = builder.connect(from, src_port, &merge_id, &format!("in_{}", i));
      }
      builder = builder.connect(&merge_id, "out", to_node, to_port);
    }
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
