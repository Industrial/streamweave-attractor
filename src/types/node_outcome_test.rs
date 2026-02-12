//! Tests for `NodeOutcome`.

use super::{NodeOutcome, OutcomeStatus};

#[test]
fn success_creates_success_outcome() {
  let o = NodeOutcome::success("done");
  assert_eq!(o.status, OutcomeStatus::Success);
  assert_eq!(o.notes.as_deref(), Some("done"));
  assert!(o.failure_reason.is_none());
  assert!(o.context_updates.is_empty());
  assert!(o.preferred_label.is_none());
  assert!(o.suggested_next_ids.is_empty());
}

#[test]
fn success_accepts_string() {
  let o = NodeOutcome::success(String::from("ok"));
  assert_eq!(o.notes.as_deref(), Some("ok"));
}

#[test]
fn fail_creates_fail_outcome() {
  let o = NodeOutcome::fail("error");
  assert_eq!(o.status, OutcomeStatus::Fail);
  assert!(o.notes.is_none());
  assert_eq!(o.failure_reason.as_deref(), Some("error"));
  assert!(o.context_updates.is_empty());
}

#[test]
fn fail_accepts_string() {
  let o = NodeOutcome::fail(String::from("boom"));
  assert_eq!(o.failure_reason.as_deref(), Some("boom"));
}
