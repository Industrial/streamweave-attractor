//! Tests for `CreateCheckpointNode`.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::CreateCheckpointNode;
use super::create_checkpoint::CreateCheckpointInput;
use super::create_checkpoint::create_checkpoint_from_input;
use super::create_checkpoint::process_create_checkpoint_item;

#[tokio::test]
async fn node_execute_skips_wrong_type() {
  let node = CreateCheckpointNode::new("cp");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new("wrong") as Arc<dyn std::any::Any + Send + Sync>)
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
async fn node_execute_creates_checkpoint() {
  let node = CreateCheckpointNode::new("cp");
  let input = CreateCheckpointInput {
    context: HashMap::new(),
    current_node_id: "run".to_string(),
    completed_nodes: vec!["start".to_string()],
  };
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
  let cp = item
    .unwrap()
    .downcast::<crate::types::Checkpoint>()
    .unwrap();
  assert_eq!(cp.current_node_id, "run");
  assert_eq!(cp.completed_nodes, vec!["start"]);
}

#[test]
fn node_trait_methods() {
  let mut node = CreateCheckpointNode::new("cp");
  assert_eq!(node.name(), "cp");
  node.set_name("checkpoint");
  assert_eq!(node.name(), "checkpoint");
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
}

#[test]
fn new_creates_node() {
  let n = CreateCheckpointNode::new("checkpoint");
  assert_eq!(n.name(), "checkpoint");
  assert!(n.has_input_port("in"));
  assert!(n.has_output_port("out"));
}

#[test]
fn create_checkpoint_from_input_builds_checkpoint() {
  let input = CreateCheckpointInput {
    context: [("k".to_string(), "v".to_string())].into_iter().collect(),
    current_node_id: "node".to_string(),
    completed_nodes: vec!["a".to_string(), "b".to_string()],
  };
  let cp = create_checkpoint_from_input(&input);
  assert_eq!(cp.current_node_id, "node");
  assert_eq!(cp.completed_nodes, vec!["a", "b"]);
  assert_eq!(cp.context.get("k"), Some(&"v".to_string()));
}

#[test]
fn process_create_checkpoint_item_returns_some_for_valid_input() {
  let input = CreateCheckpointInput {
    context: HashMap::new(),
    current_node_id: "n".to_string(),
    completed_nodes: vec![],
  };
  let item = Arc::new(input) as Arc<dyn std::any::Any + Send + Sync>;
  let cp = process_create_checkpoint_item(item);
  assert!(cp.is_some());
  assert_eq!(cp.unwrap().current_node_id, "n");
}

#[test]
fn process_create_checkpoint_item_returns_none_for_wrong_type() {
  let item = Arc::new(0_i64) as Arc<dyn std::any::Any + Send + Sync>;
  let cp = process_create_checkpoint_item(item);
  assert!(cp.is_none());
}
