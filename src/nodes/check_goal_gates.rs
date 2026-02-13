//! Check goal gates per attractor-spec §3.4.

use crate::types::{AttractorGraph, NodeOutcome, OutcomeStatus};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

/// Input for CheckGoalGatesNode.
#[derive(Clone)]
pub struct CheckGoalGatesInput {
  /// The attractor graph.
  pub graph: AttractorGraph,
  /// Outcomes keyed by node id.
  pub node_outcomes: HashMap<String, NodeOutcome>,
  /// Whether we are at the exit node.
  pub at_exit: bool,
}

/// Output: (gate_ok, retry_target).
#[derive(Clone)]
#[allow(dead_code)] // fields reserved for future wiring of CheckGoalGatesNode
pub struct CheckGoalGatesOutput {
  /// Whether all goal gates passed.
  pub gate_ok: bool,
  /// Node id to retry if gate failed.
  pub retry_target: Option<String>,
}

/// Returns true if the outcome satisfies a goal gate (Success or PartialSuccess).
#[instrument(level = "trace")]
pub(crate) fn goal_gate_passed(outcome: &NodeOutcome) -> bool {
  outcome.status == OutcomeStatus::Success || outcome.status == OutcomeStatus::PartialSuccess
}

/// Checks goal gates per attractor-spec §3.4; returns gate_ok and optional retry_target.
#[instrument(level = "trace", skip(input))]
pub(crate) fn check_goal_gates(input: &CheckGoalGatesInput) -> CheckGoalGatesOutput {
  if !input.at_exit {
    return CheckGoalGatesOutput {
      gate_ok: true,
      retry_target: None,
    };
  }
  for (node_id, outcome) in &input.node_outcomes {
    let node = match input.graph.nodes.get(node_id) {
      Some(n) => n,
      None => continue,
    };
    if node.goal_gate && !goal_gate_passed(outcome) {
      return CheckGoalGatesOutput {
        gate_ok: false,
        retry_target: node_id.clone().into(),
      };
    }
  }
  CheckGoalGatesOutput {
    gate_ok: true,
    retry_target: None,
  }
}

/// StreamWeave node that checks goal gates when at exit.
pub struct CheckGoalGatesNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`).
  output_ports: Vec<String>,
}

impl CheckGoalGatesNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for CheckGoalGatesNode {
  fn name(&self) -> &str {
    &self.name
  }

  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  fn input_port_names(&self) -> &[String] {
    &self.input_ports
  }

  fn output_port_names(&self) -> &[String] {
    &self.output_ports
  }

  fn has_input_port(&self, name: &str) -> bool {
    name == "in"
  }

  fn has_output_port(&self, name: &str) -> bool {
    name == "out"
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(16);

      tokio::spawn(async move {
        use futures::StreamExt;
        let mut s = in_stream;
        while let Some(item) = s.next().await {
          let input = match item.downcast::<CheckGoalGatesInput>() {
            Ok(arc) => (*arc).clone(),
            Err(_) => continue,
          };
          let output = check_goal_gates(&input);
          let _ = out_tx
            .send(Arc::new(output) as Arc<dyn Any + Send + Sync>)
            .await;
        }
      });

      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
