//! Result of executing a single Attractor pipeline node.

use std::collections::HashMap;

use super::OutcomeStatus;

/// Result of executing a single Attractor pipeline node.
#[derive(Debug, Clone)]
pub struct NodeOutcome {
  pub status: OutcomeStatus,
  pub notes: Option<String>,
  pub failure_reason: Option<String>,
  pub context_updates: HashMap<String, String>,
  pub preferred_label: Option<String>,
  pub suggested_next_ids: Vec<String>,
}

impl NodeOutcome {
  pub fn success(notes: impl Into<String>) -> Self {
    Self {
      status: OutcomeStatus::Success,
      notes: Some(notes.into()),
      failure_reason: None,
      context_updates: HashMap::new(),
      preferred_label: None,
      suggested_next_ids: vec![],
    }
  }

  pub fn fail(reason: impl Into<String>) -> Self {
    Self {
      status: OutcomeStatus::Fail,
      notes: None,
      failure_reason: Some(reason.into()),
      context_updates: HashMap::new(),
      preferred_label: None,
      suggested_next_ids: vec![],
    }
  }
}
