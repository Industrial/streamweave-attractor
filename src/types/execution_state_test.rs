//! Tests for `ExecutionState`.

use std::collections::HashMap;

use super::{AttractorGraph, AttractorNode, ExecutionState};

fn node(id: &str, shape: &str) -> AttractorNode {
  AttractorNode {
    id: id.to_string(),
    shape: shape.to_string(),
    handler_type: None,
    label: None,
    prompt: None,
    command: None,
    goal_gate: false,
    max_retries: 0,
  }
}

fn graph_with_nodes(nodes: Vec<AttractorNode>) -> AttractorGraph {
  let nodes_map: HashMap<String, AttractorNode> =
    nodes.into_iter().map(|n| (n.id.clone(), n)).collect();
  AttractorGraph {
    goal: "test".to_string(),
    nodes: nodes_map,
    edges: vec![],
    default_max_retry: 50,
  }
}

#[test]
fn is_done_when_at_exit() {
  let g = graph_with_nodes(vec![node("start", "Mdiamond"), node("exit", "Msquare")]);
  let state = ExecutionState {
    graph: g,
    context: HashMap::new(),
    current_node_id: "exit".to_string(),
    completed_nodes: vec![],
    node_outcomes: HashMap::new(),
    step_log: None,
  };
  assert!(state.is_done());
}

#[test]
fn is_done_false_when_at_non_exit() {
  let g = graph_with_nodes(vec![
    node("start", "Mdiamond"),
    node("mid", "ellipse"),
    node("exit", "Msquare"),
  ]);
  let state = ExecutionState {
    graph: g,
    context: HashMap::new(),
    current_node_id: "mid".to_string(),
    completed_nodes: vec![],
    node_outcomes: HashMap::new(),
    step_log: None,
  };
  assert!(!state.is_done());
}

#[test]
fn is_done_false_when_unknown_node() {
  let g = graph_with_nodes(vec![node("start", "Mdiamond"), node("exit", "Msquare")]);
  let state = ExecutionState {
    graph: g,
    context: HashMap::new(),
    current_node_id: "nonexistent".to_string(),
    completed_nodes: vec![],
    node_outcomes: HashMap::new(),
    step_log: None,
  };
  assert!(!state.is_done());
}
