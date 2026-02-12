//! Tests for `ParseDotNode`.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::parse_dot::process_dot;
use super::parse_dot::process_parse_dot_item;
use super::parse_dot::ParseDotItemResult;
use super::ParseDotNode;

#[tokio::test]
async fn node_execute_err_missing_input() {
  let node = ParseDotNode::new("parse");
  let inputs: streamweave::node::InputStreams = HashMap::new();
  let result = node.execute(inputs).await;
  assert!(result.is_err());
}

#[tokio::test]
async fn node_execute_parses_valid_dot() {
  let node = ParseDotNode::new("parse");
  let dot = r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] start -> exit }"#;
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(dot.to_string()) as Arc<dyn std::any::Any + Send + Sync>)
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
  let graph = item
    .unwrap()
    .downcast::<crate::types::AttractorGraph>()
    .unwrap();
  assert!(graph.nodes.contains_key("start"));
}

#[tokio::test]
async fn node_execute_emits_error_on_invalid_dot() {
  let node = ParseDotNode::new("parse");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new("graph foo {}".to_string()) as Arc<dyn std::any::Any + Send + Sync>)
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
  assert!(msg.contains("digraph"));
}

#[test]
fn node_trait_methods() {
  let mut node = ParseDotNode::new("parse");
  assert_eq!(node.name(), "parse");
  node.set_name("dot");
  assert_eq!(node.name(), "dot");
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[test]
fn new_creates_node() {
  let n = ParseDotNode::new("parse");
  assert_eq!(n.name(), "parse");
  assert!(n.has_input_port("in"));
  assert!(n.has_output_port("out"));
  assert!(n.has_output_port("error"));
}

#[test]
fn process_dot_parses_valid_dot() {
  let dot = r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] start -> exit }"#;
  let graph = process_dot(dot).unwrap();
  assert!(graph.nodes.contains_key("start"));
}

#[test]
fn process_dot_returns_err_on_invalid() {
  let result = process_dot("graph foo {}");
  assert!(result.is_err());
}

#[test]
fn process_parse_dot_item_graph_for_valid_dot() {
  let dot = r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] start -> exit }"#;
  let item = Arc::new(dot.to_string()) as Arc<dyn std::any::Any + Send + Sync>;
  match process_parse_dot_item(item) {
    ParseDotItemResult::Graph(g) => assert!(g.nodes.contains_key("start")),
    _ => panic!("expected Graph"),
  }
}

#[test]
fn process_parse_dot_item_parse_error_for_invalid_dot() {
  let item = Arc::new("graph foo {}".to_string()) as Arc<dyn std::any::Any + Send + Sync>;
  match process_parse_dot_item(item) {
    ParseDotItemResult::ParseError(e) => assert!(e.contains("digraph")),
    _ => panic!("expected ParseError"),
  }
}

#[test]
fn process_parse_dot_item_wrong_type_for_non_string() {
  let item = Arc::new(99_u8) as Arc<dyn std::any::Any + Send + Sync>;
  match process_parse_dot_item(item) {
    ParseDotItemResult::WrongType => {}
    _ => panic!("expected WrongType"),
  }
}
