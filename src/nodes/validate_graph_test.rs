//! Tests for `validate_graph`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{AttractorGraph, AttractorNode};
use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::validate_graph::{ValidateGraphNode, validate};

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
  let mut node = ValidateGraphNode::new("validate");
  assert_eq!(node.name(), "validate");
  node.set_name("check");
  assert_eq!(node.name(), "check");
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[test]
fn validate_ok_with_start_and_exit() {
  let g = graph(vec![node("start", "Mdiamond"), node("exit", "Msquare")]);
  assert!(validate(&g).is_ok());
}

#[test]
fn validate_err_no_start() {
  let g = graph(vec![node("exit", "Msquare")]);
  let r = validate(&g);
  assert!(r.is_err());
  assert!(r.unwrap_err().contains("start"));
}

#[test]
fn validate_err_no_exit() {
  let g = graph(vec![node("start", "Mdiamond")]);
  let r = validate(&g);
  assert!(r.is_err());
  assert!(r.unwrap_err().contains("exit"));
}

#[tokio::test]
async fn node_execute_sends_error_on_wrong_type() {
  let node = ValidateGraphNode::new("validate");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(99_i64) as Arc<dyn std::any::Any + Send + Sync>)
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
  let mut err = outputs.remove("error").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = err.next().await;
  assert!(item.is_some());
  let msg = item.unwrap().downcast::<String>().unwrap();
  assert!(msg.contains("AttractorGraph"));
}

#[tokio::test]
async fn node_execute_validates_and_forwards() {
  let g = graph(vec![node("start", "Mdiamond"), node("exit", "Msquare")]);
  let node = ValidateGraphNode::new("validate");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(g) as Arc<dyn std::any::Any + Send + Sync>)
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
  let graph = item.unwrap().downcast::<AttractorGraph>().unwrap();
  assert!(graph.nodes.contains_key("start"));
}

#[tokio::test]
async fn node_execute_emits_error_on_invalid_graph() {
  let g = graph(vec![node("exit", "Msquare")]);
  let node = ValidateGraphNode::new("validate");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(g) as Arc<dyn std::any::Any + Send + Sync>)
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
  let mut err = outputs.remove("error").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = err.next().await;
  assert!(item.is_some());
  let msg = item.unwrap().downcast::<String>().unwrap();
  assert!(msg.contains("start"));
}
