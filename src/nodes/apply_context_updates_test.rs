//! Tests for `apply_context_updates`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::NodeOutcome;
use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::apply_context_updates::{
  ApplyContextUpdatesInput, ApplyContextUpdatesNode, apply_updates,
  process_apply_context_updates_item,
};

#[tokio::test]
async fn node_execute_skips_wrong_type() {
  let node = ApplyContextUpdatesNode::new("apply");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(42_usize) as Arc<dyn std::any::Any + Send + Sync>)
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
async fn node_execute_applies_updates() {
  let node = ApplyContextUpdatesNode::new("apply");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  let input = ApplyContextUpdatesInput {
    context: HashMap::new(),
    outcome: NodeOutcome::success("ok"),
  };
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
  let ctx = item.unwrap().downcast::<HashMap<String, String>>().unwrap();
  assert!(ctx.contains_key("outcome"));
}

#[test]
fn node_trait_methods() {
  let mut node = ApplyContextUpdatesNode::new("apply");
  assert_eq!(node.name(), "apply");
  node.set_name("updated");
  assert_eq!(node.name(), "updated");
  assert_eq!(node.input_port_names(), &["in"]);
  assert_eq!(node.output_port_names(), &["out"]);
  assert!(node.has_input_port("in"));
  assert!(!node.has_input_port("x"));
  assert!(node.has_output_port("out"));
  assert!(!node.has_output_port("err"));
}

#[test]
fn apply_updates_merges_context() {
  let mut ctx = HashMap::new();
  ctx.insert("a".to_string(), "1".to_string());
  let mut outcome = NodeOutcome::success("ok");
  outcome
    .context_updates
    .insert("b".to_string(), "2".to_string());
  let input = ApplyContextUpdatesInput {
    context: ctx,
    outcome,
  };
  let result = apply_updates(&input);
  assert_eq!(result.get("a").map(String::as_str), Some("1"));
  assert_eq!(result.get("b").map(String::as_str), Some("2"));
  assert!(result.contains_key("outcome"));
}

#[test]
fn process_apply_context_updates_item_returns_some_for_valid_input() {
  let input = ApplyContextUpdatesInput {
    context: HashMap::new(),
    outcome: NodeOutcome::success("ok"),
  };
  let item = Arc::new(input) as Arc<dyn std::any::Any + Send + Sync>;
  let ctx = process_apply_context_updates_item(item);
  assert!(ctx.is_some());
  assert!(ctx.unwrap().contains_key("outcome"));
}

#[test]
fn process_apply_context_updates_item_returns_none_for_wrong_type() {
  let item = Arc::new("wrong") as Arc<dyn std::any::Any + Send + Sync>;
  assert!(process_apply_context_updates_item(item).is_none());
}

#[test]
fn apply_updates_preferred_label() {
  let mut outcome = NodeOutcome::success("ok");
  outcome.preferred_label = Some("yes".to_string());
  let input = ApplyContextUpdatesInput {
    context: HashMap::new(),
    outcome,
  };
  let result = apply_updates(&input);
  assert_eq!(
    result.get("preferred_label").map(String::as_str),
    Some("yes")
  );
}
