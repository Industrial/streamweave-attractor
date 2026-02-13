//! Tests for `select_edge`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{AttractorEdge, AttractorGraph, AttractorNode, NodeOutcome};
use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::select_edge::{
  SelectEdgeInput, SelectEdgeNode, best_by_weight_then_lexical, evaluate_condition,
  normalize_label, select_edge,
};

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

fn graph(nodes: Vec<AttractorNode>, edges: Vec<AttractorEdge>) -> AttractorGraph {
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
fn evaluate_condition_outcome_equals() {
  let mut ctx = HashMap::new();
  ctx.insert("outcome".to_string(), "Success".to_string());
  let o = NodeOutcome::success("x");
  assert!(evaluate_condition("outcome=Success", &o, &ctx));
  assert!(evaluate_condition("outcome=success", &o, &ctx));
}

#[test]
fn evaluate_condition_has_tasks_true() {
  let mut ctx = HashMap::new();
  ctx.insert("has_tasks".to_string(), "true".to_string());
  let o = NodeOutcome::success("ok");
  assert!(evaluate_condition("has_tasks=true", &o, &ctx));
}

#[test]
fn evaluate_condition_has_tasks_false() {
  let mut ctx = HashMap::new();
  ctx.insert("has_tasks".to_string(), "false".to_string());
  let o = NodeOutcome::success("ok");
  assert!(evaluate_condition("has_tasks=false", &o, &ctx));
}

#[test]
fn select_by_condition_has_tasks() {
  let mut ctx = HashMap::new();
  ctx.insert("has_tasks".to_string(), "true".to_string());
  ctx.insert("ready_task_id".to_string(), "bd-42".to_string());
  let g = graph(
    vec![
      node("check", "box"),
      node("claim", "box"),
      node("exit", "Msquare"),
    ],
    vec![
      AttractorEdge {
        from_node: "check".to_string(),
        to_node: "claim".to_string(),
        label: None,
        condition: Some("has_tasks=true".to_string()),
        weight: 0,
      },
      AttractorEdge {
        from_node: "check".to_string(),
        to_node: "exit".to_string(),
        label: None,
        condition: Some("has_tasks=false".to_string()),
        weight: 0,
      },
    ],
  );
  let input = SelectEdgeInput {
    node_id: "check".to_string(),
    outcome: NodeOutcome::success("ok"),
    context: ctx,
    graph: g,
  };
  let out = select_edge(&input);
  assert_eq!(out.next_node_id.as_deref(), Some("claim"));
}

#[test]
fn normalize_label_strips_prefixes() {
  let n = normalize_label("  [Yes]-x  ");
  assert!(n.contains("x"));
}

#[test]
fn best_by_weight_then_lexical_picks_highest_weight() {
  let edges = [
    AttractorEdge {
      from_node: "a".to_string(),
      to_node: "z".to_string(),
      label: None,
      condition: None,
      weight: 1,
    },
    AttractorEdge {
      from_node: "a".to_string(),
      to_node: "b".to_string(),
      label: None,
      condition: None,
      weight: 10,
    },
  ];
  let refs: Vec<_> = edges.iter().collect();
  let best = best_by_weight_then_lexical(refs);
  assert_eq!(best.weight, 10);
  assert_eq!(best.to_node, "b");
}

#[test]
fn node_trait_methods() {
  let mut node = SelectEdgeNode::new("sel");
  assert_eq!(node.name(), "sel");
  node.set_name("edge");
  assert_eq!(node.name(), "edge");
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
}

#[test]
fn select_unconditional_edge() {
  let g = graph(
    vec![node("a", "ellipse"), node("b", "ellipse")],
    vec![AttractorEdge {
      from_node: "a".to_string(),
      to_node: "b".to_string(),
      label: None,
      condition: None,
      weight: 0,
    }],
  );
  let input = SelectEdgeInput {
    node_id: "a".to_string(),
    outcome: NodeOutcome::success("ok"),
    context: HashMap::new(),
    graph: g,
  };
  let out = select_edge(&input);
  assert_eq!(out.next_node_id.as_deref(), Some("b"));
  assert!(!out.done);
}

#[test]
fn done_when_no_edges_and_success() {
  let g = graph(vec![node("exit", "Msquare")], vec![]);
  let input = SelectEdgeInput {
    node_id: "exit".to_string(),
    outcome: NodeOutcome::success("done"),
    context: HashMap::new(),
    graph: g,
  };
  let out = select_edge(&input);
  assert!(out.next_node_id.is_none());
  assert!(out.done);
}

#[test]
fn done_false_when_no_edges_and_fail() {
  let g = graph(vec![node("x", "box")], vec![]);
  let mut ctx = HashMap::new();
  ctx.insert("outcome".to_string(), "FAIL".to_string());
  let input = SelectEdgeInput {
    node_id: "x".to_string(),
    outcome: NodeOutcome::fail("err"),
    context: ctx,
    graph: g,
  };
  let out = select_edge(&input);
  assert!(out.next_node_id.is_none());
  assert!(!out.done);
}

#[test]
fn select_by_condition_outcome_eq() {
  let mut ctx = HashMap::new();
  ctx.insert("outcome".to_string(), "Success".to_string());
  let g = graph(
    vec![node("a", "box"), node("b", "box"), node("c", "box")],
    vec![
      AttractorEdge {
        from_node: "a".to_string(),
        to_node: "b".to_string(),
        label: None,
        condition: Some("outcome=Success".to_string()),
        weight: 5,
      },
      AttractorEdge {
        from_node: "a".to_string(),
        to_node: "c".to_string(),
        label: None,
        condition: Some("outcome=Fail".to_string()),
        weight: 10,
      },
    ],
  );
  let input = SelectEdgeInput {
    node_id: "a".to_string(),
    outcome: NodeOutcome::success("ok"),
    context: ctx,
    graph: g,
  };
  let out = select_edge(&input);
  assert_eq!(out.next_node_id.as_deref(), Some("b"));
}

#[test]
fn select_by_condition_outcome_neq() {
  let mut ctx = HashMap::new();
  ctx.insert("outcome".to_string(), "Fail".to_string());
  let g = graph(
    vec![node("a", "box"), node("b", "box")],
    vec![AttractorEdge {
      from_node: "a".to_string(),
      to_node: "b".to_string(),
      label: None,
      condition: Some("outcome!=Success".to_string()),
      weight: 0,
    }],
  );
  let input = SelectEdgeInput {
    node_id: "a".to_string(),
    outcome: NodeOutcome::fail("x"),
    context: ctx,
    graph: g,
  };
  let out = select_edge(&input);
  assert_eq!(out.next_node_id.as_deref(), Some("b"));
}

#[test]
fn select_by_preferred_label() {
  let g = graph(
    vec![node("a", "box"), node("b", "box"), node("c", "box")],
    vec![
      AttractorEdge {
        from_node: "a".to_string(),
        to_node: "b".to_string(),
        label: Some("Next".to_string()),
        condition: None,
        weight: 0,
      },
      AttractorEdge {
        from_node: "a".to_string(),
        to_node: "c".to_string(),
        label: Some("Other".to_string()),
        condition: None,
        weight: 0,
      },
    ],
  );
  let mut outcome = NodeOutcome::success("ok");
  outcome.preferred_label = Some("Next".to_string());
  let input = SelectEdgeInput {
    node_id: "a".to_string(),
    outcome,
    context: HashMap::new(),
    graph: g,
  };
  let out = select_edge(&input);
  assert_eq!(out.next_node_id.as_deref(), Some("b"));
}

#[test]
fn select_by_suggested_next_ids() {
  let g = graph(
    vec![node("a", "box"), node("b", "box"), node("c", "box")],
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
    ],
  );
  let mut outcome = NodeOutcome::success("ok");
  outcome.suggested_next_ids = vec!["c".to_string()];
  let input = SelectEdgeInput {
    node_id: "a".to_string(),
    outcome,
    context: HashMap::new(),
    graph: g,
  };
  let out = select_edge(&input);
  assert_eq!(out.next_node_id.as_deref(), Some("c"));
}

#[test]
fn select_best_by_weight_then_lexical() {
  let g = graph(
    vec![node("a", "box"), node("b", "box"), node("c", "box")],
    vec![
      AttractorEdge {
        from_node: "a".to_string(),
        to_node: "b".to_string(),
        label: None,
        condition: None,
        weight: 5,
      },
      AttractorEdge {
        from_node: "a".to_string(),
        to_node: "c".to_string(),
        label: None,
        condition: None,
        weight: 10,
      },
    ],
  );
  let input = SelectEdgeInput {
    node_id: "a".to_string(),
    outcome: NodeOutcome::success("ok"),
    context: HashMap::new(),
    graph: g,
  };
  let out = select_edge(&input);
  assert_eq!(out.next_node_id.as_deref(), Some("c"));
}

#[tokio::test]
async fn node_execute_skips_wrong_type() {
  let sel_node = SelectEdgeNode::new("sel");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(false) as Arc<dyn std::any::Any + Send + Sync>)
    .await
    .unwrap();
  drop(tx);
  let mut inputs: streamweave::node::InputStreams = HashMap::new();
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(rx))
      as std::pin::Pin<
        Box<dyn futures::Stream<Item = Arc<dyn std::any::Any + Send + Sync>> + Send>,
      >,
  );
  let mut outputs = sel_node.execute(inputs).await.unwrap();
  let mut out = outputs.remove("out").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = out.next().await;
  assert!(item.is_none());
}

#[tokio::test]
async fn node_execute_selects_edge() {
  let g = graph(
    vec![node("a", "box"), node("b", "box")],
    vec![AttractorEdge {
      from_node: "a".to_string(),
      to_node: "b".to_string(),
      label: None,
      condition: None,
      weight: 0,
    }],
  );
  let input = SelectEdgeInput {
    node_id: "a".to_string(),
    outcome: NodeOutcome::success("ok"),
    context: HashMap::new(),
    graph: g,
  };
  let sel_node = SelectEdgeNode::new("sel");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(input) as Arc<dyn std::any::Any + Send + Sync>)
    .await
    .unwrap();
  drop(tx);
  let mut inputs: streamweave::node::InputStreams = HashMap::new();
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(rx))
      as std::pin::Pin<
        Box<dyn futures::Stream<Item = Arc<dyn std::any::Any + Send + Sync>> + Send>,
      >,
  );
  let mut outputs = sel_node.execute(inputs).await.unwrap();
  let mut out = outputs.remove("out").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = out.next().await;
  assert!(item.is_some());
  let result = item
    .unwrap()
    .downcast::<super::select_edge::SelectEdgeOutput>()
    .unwrap();
  assert_eq!(result.next_node_id.as_deref(), Some("b"));
}
