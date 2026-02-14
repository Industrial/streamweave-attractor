//! Tests for `AttractorExecutionLoopNode`.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::AttractorExecutionLoopNode;
use super::execution_loop::{RunLoopResult, apply_context_updates, run_execution_loop_once};
use crate::types::{AttractorGraph, AttractorNode, ExecutionState, NodeOutcome};

#[test]
fn apply_context_updates_merges_outcome() {
  let mut ctx = HashMap::new();
  ctx.insert("a".to_string(), "1".to_string());
  let mut o = NodeOutcome::success("ok");
  o.context_updates.insert("b".to_string(), "2".to_string());
  o.preferred_label = Some("yes".to_string());
  apply_context_updates(&mut ctx, &o);
  assert_eq!(ctx.get("a").map(String::as_str), Some("1"));
  assert_eq!(ctx.get("b").map(String::as_str), Some("2"));
  assert!(ctx.contains_key("outcome"));
  assert_eq!(ctx.get("preferred_label").map(String::as_str), Some("yes"));
}

#[test]
fn node_trait_methods() {
  let mut node = AttractorExecutionLoopNode::new("exec");
  assert_eq!(node.name(), "exec");
  node.set_name("loop");
  assert_eq!(node.name(), "loop");
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn node_execute_err_missing_input() {
  let node = AttractorExecutionLoopNode::new("exec");
  let inputs: streamweave::node::InputStreams = HashMap::new();
  let result = node.execute(inputs).await;
  assert!(result.is_err());
}

#[tokio::test]
async fn node_execute_err_node_not_found() {
  use crate::types::{AttractorGraph, AttractorNode, ExecutionState};
  use std::collections::HashMap;

  let mut nodes = HashMap::new();
  nodes.insert(
    "start".to_string(),
    AttractorNode {
      id: "start".to_string(),
      shape: "Mdiamond".to_string(),
      handler_type: Some("start".to_string()),
      label: None,
      prompt: None,
      command: None,
      goal_gate: false,
      max_retries: 0,
    },
  );
  nodes.insert(
    "exit".to_string(),
    AttractorNode {
      id: "exit".to_string(),
      shape: "Msquare".to_string(),
      handler_type: Some("exit".to_string()),
      label: None,
      prompt: None,
      command: None,
      goal_gate: false,
      max_retries: 0,
    },
  );
  let graph = AttractorGraph {
    goal: "test".to_string(),
    nodes,
    edges: vec![],
    default_max_retry: 50,
  };
  let mut context = HashMap::new();
  context.insert("goal".to_string(), "test".to_string());
  let state = ExecutionState {
    graph: graph.clone(),
    context,
    current_node_id: "nonexistent".to_string(),
    completed_nodes: vec![],
    node_outcomes: HashMap::new(),
    step_log: None,
  };
  let node = AttractorExecutionLoopNode::new("exec");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(state) as Arc<dyn std::any::Any + Send + Sync>)
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
  let mut result = node.execute(inputs).await.unwrap();
  let mut err = result.remove("error").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = err.next().await;
  assert!(item.is_some());
  let msg = item.unwrap().downcast::<String>().unwrap();
  assert!(msg.contains("Node not found"));
}

#[tokio::test]
async fn node_execute_err_wrong_input_type() {
  let node = AttractorExecutionLoopNode::new("exec");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new("not ExecutionState") as Arc<dyn std::any::Any + Send + Sync>)
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
  let mut result = node.execute(inputs).await.unwrap();
  let mut err = result.remove("error").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = err.next().await;
  assert!(item.is_some());
  let msg = item.unwrap().downcast::<String>().unwrap();
  assert!(msg.contains("ExecutionState"));
}

#[test]
fn new_creates_node() {
  let n = AttractorExecutionLoopNode::new("execute");
  assert_eq!(n.name(), "execute");
  assert!(n.has_input_port("in"));
  assert!(n.has_output_port("out"));
  assert!(n.has_output_port("error"));
}

#[test]
fn run_execution_loop_once_returns_ok_for_simple_pipeline() {
  let dot = r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] start -> exit }"#;
  let graph = crate::dot_parser::parse_dot(dot).unwrap();
  let mut context = HashMap::new();
  context.insert("goal".to_string(), graph.goal.clone());
  let mut state = ExecutionState {
    graph: graph.clone(),
    context,
    current_node_id: "start".to_string(),
    completed_nodes: vec![],
    node_outcomes: HashMap::new(),
    step_log: None,
  };
  match run_execution_loop_once(&mut state) {
    RunLoopResult::Ok(r) => {
      assert_eq!(r.completed_nodes, vec!["start", "exit"]);
    }
    RunLoopResult::Err(e) => panic!("expected Ok, got Err: {}", e),
  }
}

#[test]
fn run_execution_loop_once_returns_err_when_node_not_found() {
  let mut nodes = HashMap::new();
  nodes.insert(
    "start".to_string(),
    AttractorNode {
      id: "start".to_string(),
      shape: "Mdiamond".to_string(),
      handler_type: Some("start".to_string()),
      label: None,
      prompt: None,
      command: None,
      goal_gate: false,
      max_retries: 0,
    },
  );
  let graph = AttractorGraph {
    goal: "test".to_string(),
    nodes,
    edges: vec![],
    default_max_retry: 50,
  };
  let mut context = HashMap::new();
  context.insert("goal".to_string(), "test".to_string());
  let mut state = ExecutionState {
    graph,
    context,
    current_node_id: "nonexistent".to_string(),
    completed_nodes: vec![],
    node_outcomes: HashMap::new(),
    step_log: None,
  };
  match run_execution_loop_once(&mut state) {
    RunLoopResult::Err(e) => assert!(e.contains("Node not found")),
    RunLoopResult::Ok(_) => panic!("expected Err"),
  }
}

#[test]
fn run_execution_loop_once_records_steps_when_step_log_is_some() {
  let dot = r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] start -> exit }"#;
  let graph = crate::dot_parser::parse_dot(dot).unwrap();
  let mut context = HashMap::new();
  context.insert("goal".to_string(), graph.goal.clone());
  let step_log = Some(Vec::new());
  let mut state = ExecutionState {
    graph: graph.clone(),
    context,
    current_node_id: "start".to_string(),
    completed_nodes: vec![],
    node_outcomes: HashMap::new(),
    step_log: step_log.clone(),
  };
  match run_execution_loop_once(&mut state) {
    RunLoopResult::Ok(r) => {
      assert_eq!(r.completed_nodes, vec!["start", "exit"]);
    }
    RunLoopResult::Err(e) => panic!("expected Ok, got Err: {}", e),
  }
  let log = state.step_log.unwrap();
  assert_eq!(log.len(), 2, "expected two steps (start, exit)");
  assert_eq!(log[0].step, 1);
  assert_eq!(log[0].node_id, "start");
  assert_eq!(log[0].next_node_id.as_deref(), Some("exit"));
  assert_eq!(log[0].completed_nodes_after, vec!["start"]);
  assert_eq!(log[1].step, 2);
  assert_eq!(log[1].node_id, "exit");
  assert_eq!(log[1].next_node_id, None);
  assert_eq!(log[1].completed_nodes_after, vec!["start", "exit"]);
}
