//! Tests for `OutcomeStatus`.

use super::OutcomeStatus;

#[test]
fn display_success() {
  assert_eq!(OutcomeStatus::Success.to_string(), "success");
}

#[test]
fn display_partial_success() {
  assert_eq!(OutcomeStatus::PartialSuccess.to_string(), "partial_success");
}

#[test]
fn display_error() {
  assert_eq!(OutcomeStatus::Error.to_string(), "error");
}

#[test]
fn display_retry() {
  assert_eq!(OutcomeStatus::Retry.to_string(), "retry");
}

#[test]
fn eq_variants() {
  assert_eq!(OutcomeStatus::Success, OutcomeStatus::Success);
  assert_ne!(OutcomeStatus::Success, OutcomeStatus::Error);
}
