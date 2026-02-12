//! Outcome status for a node execution.

use std::fmt;

/// Outcome status for a node execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeStatus {
  Success,
  PartialSuccess,
  Fail,
  Retry,
}

impl fmt::Display for OutcomeStatus {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      OutcomeStatus::Success => write!(f, "success"),
      OutcomeStatus::PartialSuccess => write!(f, "partial_success"),
      OutcomeStatus::Fail => write!(f, "fail"),
      OutcomeStatus::Retry => write!(f, "retry"),
    }
  }
}
