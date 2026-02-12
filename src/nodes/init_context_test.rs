//! Tests for `InitContextNode`.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::init_context::create_initial_state;
use super::init_context::process_init_context_item;
use super::InitContextNode;

#[tokio::test]
async fn node_execute_skips_wrong_type() {
  let node = InitContextNode::new("init");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(0_u32) as Arc<dyn std::any::Any + Send + Sync>)
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
async fn node_execute_creates_execution_state() {
  let dot = r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] start -> exit }"#;
  let graph = crate::dot_parser::parse_dot(dot).unwrap();
  let node = InitContextNode::new("init");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(graph) as Arc<dyn std::any::Any + Send + Sync>)
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
  let state = item
    .unwrap()
    .downcast::<crate::types::ExecutionState>()
    .unwrap();
  assert_eq!(state.current_node_id, "start");
  assert!(state.context.contains_key("goal"));
}

#[test]
fn node_trait_methods() {
  let mut node = InitContextNode::new("init");
  assert_eq!(node.name(), "init");
  node.set_name("context");
  assert_eq!(node.name(), "context");
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
}

#[test]
fn new_creates_node() {
  let n = InitContextNode::new("init");
  assert_eq!(n.name(), "init");
  assert!(n.has_input_port("in"));
  assert!(n.has_output_port("out"));
}

#[test]
fn create_initial_state_populates_context_and_start() {
  let dot = r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] start -> exit }"#;
  let graph = crate::dot_parser::parse_dot(dot).unwrap();
  let state = create_initial_state(graph);
  assert_eq!(state.current_node_id, "start");
  assert!(state.context.contains_key("goal"));
  assert!(state.context.contains_key("graph.goal"));
  assert!(state.completed_nodes.is_empty());
}

#[test]
fn process_init_context_item_returns_some_for_graph() {
  let dot = r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] start -> exit }"#;
  let graph = crate::dot_parser::parse_dot(dot).unwrap();
  let item = Arc::new(graph) as Arc<dyn std::any::Any + Send + Sync>;
  let state = process_init_context_item(item);
  assert!(state.is_some());
  assert_eq!(state.unwrap().current_node_id, "start");
}

#[test]
fn process_init_context_item_returns_none_for_wrong_type() {
  let item = Arc::new("not a graph".to_string()) as Arc<dyn std::any::Any + Send + Sync>;
  let state = process_init_context_item(item);
  assert!(state.is_none());
}
