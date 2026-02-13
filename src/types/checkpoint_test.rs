//! Tests for `Checkpoint`.

use std::collections::HashMap;

use super::Checkpoint;

#[test]
fn checkpoint_roundtrip_serde() {
  let mut ctx = HashMap::new();
  ctx.insert("goal".to_string(), "build".to_string());
  let cp = Checkpoint {
    context: ctx,
    current_node_id: "run".to_string(),
    completed_nodes: vec!["start".to_string(), "run".to_string()],
  };
  let json = serde_json::to_string(&cp).unwrap();
  let cp2: Checkpoint = serde_json::from_str(&json).unwrap();
  assert_eq!(cp2.current_node_id, cp.current_node_id);
  assert_eq!(cp2.completed_nodes, cp.completed_nodes);
  assert_eq!(cp2.context.get("goal").map(String::as_str), Some("build"));
}

#[test]
fn construct_checkpoint() {
  let mut ctx = HashMap::new();
  ctx.insert("key".to_string(), "val".to_string());
  let cp = Checkpoint {
    context: ctx.clone(),
    current_node_id: "node1".to_string(),
    completed_nodes: vec!["start".to_string(), "node1".to_string()],
  };
  assert_eq!(cp.context.get("key").map(String::as_str), Some("val"));
  assert_eq!(cp.current_node_id, "node1");
  assert_eq!(cp.completed_nodes.len(), 2);
}

#[test]
fn clone_checkpoint() {
  let mut ctx = HashMap::new();
  ctx.insert("x".to_string(), "y".to_string());
  let cp = Checkpoint {
    context: ctx,
    current_node_id: "here".to_string(),
    completed_nodes: vec!["a".to_string()],
  };
  let c = cp.clone();
  assert_eq!(c.current_node_id, cp.current_node_id);
  assert_eq!(c.completed_nodes, cp.completed_nodes);
}
