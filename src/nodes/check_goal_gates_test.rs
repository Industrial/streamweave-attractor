//! Tests for `check_goal_gates`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{AttractorGraph, AttractorNode, NodeOutcome};
use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::check_goal_gates::{
  goal_gate_passed, CheckGoalGatesInput, CheckGoalGatesNode, check_goal_gates,
};

fn node(id: &str, goal_gate: bool) -> AttractorNode {
  AttractorNode {
    id: id.to_string(),
    shape: "ellipse".to_string(),
    handler_type: None,
    label: None,
    prompt: None,
    goal_gate,
    max_retries: 0,
  }
}

fn graph(nodes: Vec<AttractorNode>) -> AttractorGraph {
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
fn node_trait_methods() {
  let mut node = CheckGoalGatesNode::new("check");
  assert_eq!(node.name(), "check");
  node.set_name("gates");
  assert_eq!(node.name(), "gates");
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
}

#[test]
fn goal_gate_passed_for_success_and_partial_success() {
  assert!(goal_gate_passed(&NodeOutcome::success("ok")));
  let mut o = NodeOutcome::success("x");
  o.status = crate::types::OutcomeStatus::PartialSuccess;
  assert!(goal_gate_passed(&o));
}

#[test]
fn goal_gate_passed_false_for_fail() {
  assert!(!goal_gate_passed(&NodeOutcome::fail("err")));
}

#[test]
fn gate_ok_when_not_at_exit() {
  let g = graph(vec![node("a", true)]);
  let mut outcomes = HashMap::new();
  outcomes.insert("a".to_string(), NodeOutcome::fail("err"));
  let input = CheckGoalGatesInput {
    graph: g,
    node_outcomes: outcomes,
    at_exit: false,
  };
  let out = check_goal_gates(&input);
  assert!(out.gate_ok);
}

#[test]
fn gate_ok_when_goal_gate_success() {
  let g = graph(vec![node("gate", true)]);
  let mut outcomes = HashMap::new();
  outcomes.insert("gate".to_string(), NodeOutcome::success("ok"));
  let input = CheckGoalGatesInput {
    graph: g,
    node_outcomes: outcomes,
    at_exit: true,
  };
  let out = check_goal_gates(&input);
  assert!(out.gate_ok);
}

#[test]
fn gate_not_ok_when_goal_gate_fail() {
  let g = graph(vec![node("gate", true)]);
  let mut outcomes = HashMap::new();
  outcomes.insert("gate".to_string(), NodeOutcome::fail("err"));
  let input = CheckGoalGatesInput {
    graph: g,
    node_outcomes: outcomes,
    at_exit: true,
  };
  let out = check_goal_gates(&input);
  assert!(!out.gate_ok);
  assert_eq!(out.retry_target.as_deref(), Some("gate"));
}

#[tokio::test]
async fn node_execute_skips_wrong_type() {
  let node = CheckGoalGatesNode::new("check");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(0_i32) as Arc<dyn std::any::Any + Send + Sync>)
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
  let mut outputs = node.execute(inputs).await.unwrap();
  let mut out = outputs.remove("out").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = out.next().await;
  assert!(item.is_none());
}

#[tokio::test]
async fn node_execute_checks_gates() {
  let g = graph(vec![node("gate", true)]);
  let mut outcomes = HashMap::new();
  outcomes.insert("gate".to_string(), NodeOutcome::success("ok"));
  let input = CheckGoalGatesInput {
    graph: g,
    node_outcomes: outcomes,
    at_exit: true,
  };
  let node = CheckGoalGatesNode::new("check");
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
  let mut outputs = node.execute(inputs).await.unwrap();
  let mut out = outputs.remove("out").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = out.next().await;
  assert!(item.is_some());
  let result = item
    .unwrap()
    .downcast::<super::check_goal_gates::CheckGoalGatesOutput>()
    .unwrap();
  assert!(result.gate_ok);
}
