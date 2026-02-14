//! Tests for `ResumeState`.

use std::collections::HashMap;

use super::ResumeState;

#[test]
fn resume_state_roundtrip_serde() {
  let mut ctx = HashMap::new();
  ctx.insert("goal".to_string(), "build".to_string());
  let st = ResumeState {
    context: ctx,
    current_node_id: "run".to_string(),
    completed_nodes: vec!["start".to_string(), "run".to_string()],
  };
  let json = serde_json::to_string(&st).unwrap();
  let st2: ResumeState = serde_json::from_str(&json).unwrap();
  assert_eq!(st2.current_node_id, st.current_node_id);
  assert_eq!(st2.completed_nodes, st.completed_nodes);
  assert_eq!(st2.context.get("goal").map(String::as_str), Some("build"));
}

#[test]
fn construct_resume_state() {
  let mut ctx = HashMap::new();
  ctx.insert("key".to_string(), "val".to_string());
  let st = ResumeState {
    context: ctx.clone(),
    current_node_id: "node1".to_string(),
    completed_nodes: vec!["start".to_string(), "node1".to_string()],
  };
  assert_eq!(st.context.get("key").map(String::as_str), Some("val"));
  assert_eq!(st.current_node_id, "node1");
  assert_eq!(st.completed_nodes.len(), 2);
}

#[test]
fn clone_resume_state() {
  let mut ctx = HashMap::new();
  ctx.insert("x".to_string(), "y".to_string());
  let st = ResumeState {
    context: ctx,
    current_node_id: "here".to_string(),
    completed_nodes: vec!["a".to_string()],
  };
  let c = st.clone();
  assert_eq!(c.current_node_id, st.current_node_id);
  assert_eq!(c.completed_nodes, st.completed_nodes);
}
