//! Tests for `execute_handler`.

use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{AttractorGraph, AttractorNode, OutcomeStatus};
use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::execute_handler::{
  build_codergen_outcome, ExecuteHandlerInput, ExecuteHandlerNode, execute_handler,
};

fn node(id: &str, handler_type: Option<&str>) -> AttractorNode {
  AttractorNode {
    id: id.to_string(),
    shape: "ellipse".to_string(),
    handler_type: handler_type.map(String::from),
    label: None,
    prompt: None,
    goal_gate: false,
    max_retries: 0,
  }
}

fn empty_graph() -> AttractorGraph {
  AttractorGraph {
    goal: "test".to_string(),
    nodes: HashMap::new(),
    edges: vec![],
    default_max_retry: 50,
  }
}

#[test]
fn node_trait_methods() {
  let mut node = ExecuteHandlerNode::new("exec");
  assert_eq!(node.name(), "exec");
  node.set_name("handler");
  assert_eq!(node.name(), "handler");
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[test]
fn build_codergen_outcome_includes_last_stage() {
  let n = node("run", Some("codergen"));
  let o = build_codergen_outcome(&n);
  assert_eq!(o.status, OutcomeStatus::Success);
  assert_eq!(o.context_updates.get("last_stage").map(String::as_str), Some("run"));
}

#[test]
fn start_handler() {
  let input = ExecuteHandlerInput {
    node: node("start", Some("start")),
    context: HashMap::new(),
    graph: empty_graph(),
  };
  let out = execute_handler(&input).unwrap();
  assert_eq!(out.status, OutcomeStatus::Success);
  assert_eq!(out.notes.as_deref(), Some("Start"));
}

#[test]
fn exit_handler() {
  let input = ExecuteHandlerInput {
    node: node("exit", Some("exit")),
    context: HashMap::new(),
    graph: empty_graph(),
  };
  let out = execute_handler(&input).unwrap();
  assert_eq!(out.status, OutcomeStatus::Success);
  assert_eq!(out.notes.as_deref(), Some("Exit"));
}

#[test]
fn codergen_handler() {
  let input = ExecuteHandlerInput {
    node: node("run", Some("codergen")),
    context: HashMap::new(),
    graph: empty_graph(),
  };
  let out = execute_handler(&input).unwrap();
  assert_eq!(out.status, OutcomeStatus::Success);
  assert_eq!(
    out.context_updates.get("last_stage").map(String::as_str),
    Some("run")
  );
}

#[test]
fn unknown_handler_stub() {
  let input = ExecuteHandlerInput {
    node: node("x", Some("custom.handler")),
    context: HashMap::new(),
    graph: empty_graph(),
  };
  let out = execute_handler(&input).unwrap();
  assert_eq!(out.status, OutcomeStatus::Success);
  assert!(out.notes.as_deref().unwrap().contains("custom.handler"));
}

#[tokio::test]
async fn node_execute_sends_error_on_wrong_type() {
  let handler_node = ExecuteHandlerNode::new("exec");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(99_i32) as Arc<dyn std::any::Any + Send + Sync>)
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
  let mut outputs = handler_node.execute(inputs).await.unwrap();
  let mut err = outputs.remove("error").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = err.next().await;
  assert!(item.is_some());
  let msg = item.unwrap().downcast::<String>().unwrap();
  assert!(msg.contains("ExecuteHandlerInput"));
}

#[tokio::test]
async fn node_execute_runs_handler() {
  let handler_node = ExecuteHandlerNode::new("exec");
  let input = ExecuteHandlerInput {
    node: node("run", Some("start")),
    context: HashMap::new(),
    graph: empty_graph(),
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
  let mut outputs = handler_node.execute(inputs).await.unwrap();
  let mut out = outputs.remove("out").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = out.next().await;
  assert!(item.is_some());
  let outcome = item
    .unwrap()
    .downcast::<crate::types::NodeOutcome>()
    .unwrap();
  assert_eq!(outcome.status, OutcomeStatus::Success);
}
