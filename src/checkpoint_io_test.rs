//! Tests for checkpoint save/load.

use crate::checkpoint_io::{CHECKPOINT_FILENAME, load_checkpoint, save_checkpoint};
use crate::types::Checkpoint;
use std::collections::HashMap;

#[test]
fn roundtrip_save_load() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join(CHECKPOINT_FILENAME);
  let mut ctx = HashMap::new();
  ctx.insert("k".to_string(), "v".to_string());
  let cp = Checkpoint {
    context: ctx,
    current_node_id: "node1".to_string(),
    completed_nodes: vec!["start".to_string(), "node1".to_string()],
  };
  save_checkpoint(&path, &cp).unwrap();
  assert!(path.exists());
  let loaded = load_checkpoint(&path).unwrap();
  assert_eq!(loaded.current_node_id, cp.current_node_id);
  assert_eq!(loaded.completed_nodes, cp.completed_nodes);
  assert_eq!(loaded.context.get("k").map(String::as_str), Some("v"));
}

#[test]
fn load_missing_file_returns_error() {
  let dir = tempfile::tempdir().unwrap();
  let path = dir.path().join("nonexistent.json");
  let r = load_checkpoint(&path);
  assert!(r.is_err());
}
