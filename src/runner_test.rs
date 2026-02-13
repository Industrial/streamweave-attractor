//! Tests for `runner`.

use std::fs;

use crate::agent_run::read_outcome_json;

#[test]
fn read_outcome_json_returns_context_updates() {
  let dir = tempfile::tempdir().unwrap();
  let stage_dir = dir.path();
  let outcome_path = stage_dir.join("outcome.json");
  fs::write(
    &outcome_path,
    r#"{"context_updates":{"has_tasks":"true","ready_task_id":"streamweave-attractor-d32"}}"#,
  )
  .unwrap();

  let updates = read_outcome_json(Some(stage_dir.to_str().unwrap()));
  assert!(updates.is_some());
  let updates = updates.unwrap();
  assert_eq!(updates.get("has_tasks").map(String::as_str), Some("true"));
  assert_eq!(
    updates.get("ready_task_id").map(String::as_str),
    Some("streamweave-attractor-d32")
  );
}

#[test]
fn read_outcome_json_returns_none_when_file_missing() {
  let dir = tempfile::tempdir().unwrap();
  let stage_dir = dir.path();
  assert!(!stage_dir.join("outcome.json").exists());

  let updates = read_outcome_json(Some(stage_dir.to_str().unwrap()));
  assert!(updates.is_none());
}
