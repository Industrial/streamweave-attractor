//! Tests for `AttractorGraph`.

use std::collections::HashMap;

use super::{AttractorEdge, AttractorGraph, AttractorNode};

fn node(id: &str, shape: &str) -> AttractorNode {
  AttractorNode {
    id: id.to_string(),
    shape: shape.to_string(),
    handler_type: None,
    label: None,
    prompt: None,
    goal_gate: false,
    max_retries: 0,
  }
}

fn graph_with_nodes(nodes: Vec<AttractorNode>, edges: Vec<AttractorEdge>) -> AttractorGraph {
  let nodes_map: HashMap<String, AttractorNode> =
    nodes.into_iter().map(|n| (n.id.clone(), n)).collect();
  AttractorGraph {
    goal: "test".to_string(),
    nodes: nodes_map,
    edges,
    default_max_retry: 50,
  }
}

#[test]
fn find_start_by_shape() {
  let g = graph_with_nodes(
    vec![
      node("a", "ellipse"),
      node("start", "Mdiamond"),
      node("b", "ellipse"),
    ],
    vec![],
  );
  let start = g.find_start().unwrap();
  assert_eq!(start.id, "start");
}

#[test]
fn find_start_none() {
  let g = graph_with_nodes(vec![node("a", "ellipse"), node("b", "ellipse")], vec![]);
  assert!(g.find_start().is_none());
}

#[test]
fn find_exit_by_shape() {
  let g = graph_with_nodes(
    vec![
      node("a", "ellipse"),
      node("exit", "Msquare"),
      node("b", "ellipse"),
    ],
    vec![],
  );
  let exit = g.find_exit().unwrap();
  assert_eq!(exit.id, "exit");
}

#[test]
fn find_exit_none() {
  let g = graph_with_nodes(vec![node("a", "ellipse"), node("b", "ellipse")], vec![]);
  assert!(g.find_exit().is_none());
}

#[test]
fn outgoing_edges() {
  let g = graph_with_nodes(
    vec![
      node("a", "ellipse"),
      node("b", "ellipse"),
      node("c", "ellipse"),
    ],
    vec![
      AttractorEdge {
        from_node: "a".to_string(),
        to_node: "b".to_string(),
        label: None,
        condition: None,
        weight: 0,
      },
      AttractorEdge {
        from_node: "a".to_string(),
        to_node: "c".to_string(),
        label: None,
        condition: None,
        weight: 0,
      },
      AttractorEdge {
        from_node: "b".to_string(),
        to_node: "c".to_string(),
        label: None,
        condition: None,
        weight: 0,
      },
    ],
  );
  let out = g.outgoing_edges("a");
  assert_eq!(out.len(), 2);
  assert!(out.iter().any(|e| e.to_node == "b"));
  assert!(out.iter().any(|e| e.to_node == "c"));
}

#[test]
fn outgoing_edges_empty() {
  let g = graph_with_nodes(
    vec![node("a", "ellipse"), node("b", "ellipse")],
    vec![AttractorEdge {
      from_node: "b".to_string(),
      to_node: "a".to_string(),
      label: None,
      condition: None,
      weight: 0,
    }],
  );
  let out = g.outgoing_edges("a");
  assert!(out.is_empty());
}
