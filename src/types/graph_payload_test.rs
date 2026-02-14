//! Tests for `GraphPayload`.

use std::collections::HashMap;

use super::{Checkpoint, GraphPayload};

#[test]
fn initial_sets_start_node_and_empty_completed() {
  let mut ctx = HashMap::new();
  ctx.insert("goal".to_string(), "build".to_string());
  let p = GraphPayload::initial(ctx, "start");
  assert_eq!(p.current_node_id, "start");
  assert!(p.completed_nodes.is_empty());
  assert!(p.outcome.is_none());
  assert_eq!(p.context.get("goal").map(String::as_str), Some("build"));
}

#[test]
fn from_checkpoint_restores_context_and_completed() {
  let mut ctx = HashMap::new();
  ctx.insert("k".to_string(), "v".to_string());
  let cp = Checkpoint {
    context: ctx,
    current_node_id: "run".to_string(),
    completed_nodes: vec!["start".to_string(), "run".to_string()],
  };
  let p = GraphPayload::from_checkpoint(&cp);
  assert_eq!(p.context.get("k").map(String::as_str), Some("v"));
  assert_eq!(p.current_node_id, "run");
  assert_eq!(p.completed_nodes, &["start", "run"]);
  assert!(p.outcome.is_none());
}
