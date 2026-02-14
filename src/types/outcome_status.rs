//! Outcome status for a node execution.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Outcome status for a node execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus {
  Success,
  PartialSuccess,
  Error, // was Fail
  Retry,
}

impl fmt::Display for OutcomeStatus {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      OutcomeStatus::Success => write!(f, "success"),
      OutcomeStatus::PartialSuccess => write!(f, "partial_success"),
      OutcomeStatus::Error => write!(f, "error"),
      OutcomeStatus::Retry => write!(f, "retry"),
    }
  }
}
