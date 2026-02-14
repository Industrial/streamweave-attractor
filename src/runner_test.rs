//! Tests for `runner`.

use std::fs;

use crate::agent_run::read_outcome_file;

fn context_updates_from_file(
  stage_dir: &std::path::Path,
) -> Option<std::collections::HashMap<String, String>> {
  read_outcome_file(Some(stage_dir)).map(|(_, updates)| updates)
}

#[test]
fn read_outcome_file_returns_context_updates() {
  let dir = tempfile::tempdir().unwrap();
  let stage_dir = dir.path();
  let outcome_path = stage_dir.join("outcome.json");
  fs::write(
    &outcome_path,
    r#"{"context_updates":{"has_tasks":"true","ready_task_id":"streamweave-attractor-d32"}}"#,
  )
  .unwrap();

  let updates = context_updates_from_file(stage_dir);
  assert!(updates.is_some());
  let updates = updates.unwrap();
  assert_eq!(updates.get("has_tasks").map(String::as_str), Some("true"));
  assert_eq!(
    updates.get("ready_task_id").map(String::as_str),
    Some("streamweave-attractor-d32")
  );
}

#[test]
fn read_outcome_file_returns_none_when_file_missing() {
  let dir = tempfile::tempdir().unwrap();
  let stage_dir = dir.path();
  assert!(!stage_dir.join("outcome.json").exists());

  let updates = context_updates_from_file(stage_dir);
  assert!(updates.is_none());
}

#[test]
fn read_outcome_file_returns_none_when_invalid_json() {
  let dir = tempfile::tempdir().unwrap();
  let outcome_path = dir.path().join("outcome.json");
  std::fs::write(&outcome_path, "not json").unwrap();
  let updates = context_updates_from_file(dir.path());
  assert!(updates.is_none());
}

#[test]
fn read_outcome_file_returns_empty_map_when_no_context_updates_key() {
  let dir = tempfile::tempdir().unwrap();
  let outcome_path = dir.path().join("outcome.json");
  std::fs::write(&outcome_path, r#"{"other":{}}"#).unwrap();
  let updates = context_updates_from_file(dir.path());
  assert!(updates.is_some());
  assert!(updates.unwrap().is_empty());
}
