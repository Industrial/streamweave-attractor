//! Execute a single node handler (start, exit, codergen stub).

use crate::types::{AttractorGraph, AttractorNode, NodeOutcome, OutcomeStatus, RunContext};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

/// Input bundle for ExecuteHandlerNode.
#[derive(Clone)]
#[allow(dead_code)] // context and graph reserved for future handler implementations
pub struct ExecuteHandlerInput {
  /// The node whose handler to execute.
  pub node: AttractorNode,
  /// Current run context (reserved for future handler use).
  pub context: RunContext,
  /// The attractor graph (reserved for future handler use).
  pub graph: AttractorGraph,
}

/// StreamWeave node that executes the handler for one Attractor pipeline node.
#[allow(dead_code)] // reserved for interpretive pipeline
pub struct ExecuteHandlerNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`, `error`).
  output_ports: Vec<String>,
}

impl ExecuteHandlerNode {
  /// Creates a new ExecuteHandlerNode with the given display name.
  #[allow(dead_code)]
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string(), "error".to_string()],
    }
  }
}

/// Builds a codergen NodeOutcome for the given node.
#[instrument(level = "trace")]
pub(crate) fn build_codergen_outcome(node: &AttractorNode) -> NodeOutcome {
  let mut updates = HashMap::new();
  updates.insert("last_stage".to_string(), node.id.clone());
  NodeOutcome {
    status: OutcomeStatus::Success,
    notes: Some(format!("Stage completed: {}", node.id)),
    failure_reason: None,
    context_updates: updates,
    preferred_label: None,
    suggested_next_ids: vec![],
  }
}

/// Executes the handler for the given node (start, exit, codergen stub, etc.) and returns the outcome.
#[instrument(level = "trace", skip(input))]
pub(crate) fn execute_handler(input: &ExecuteHandlerInput) -> Result<NodeOutcome, String> {
  let handler = input.node.handler_type.as_deref().unwrap_or("codergen");
  match handler {
    "start" => Ok(NodeOutcome::success("Start")),
    "exit" => Ok(NodeOutcome::success("Exit")),
    "codergen" => Ok(build_codergen_outcome(&input.node)),
    _ => Ok(NodeOutcome::success(format!("Handler {} (stub)", handler))),
  }
}

#[async_trait]
impl Node for ExecuteHandlerNode {
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
    name == "out" || name == "error"
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
      let (err_tx, err_rx) = tokio::sync::mpsc::channel(16);

      tokio::spawn(async move {
        use futures::StreamExt;
        let mut s = in_stream;
        while let Some(item) = s.next().await {
          let input = match item.downcast::<ExecuteHandlerInput>() {
            Ok(arc) => (*arc).clone(),
            Err(_) => {
              let _ = err_tx
                .send(Arc::new("Expected ExecuteHandlerInput".to_string())
                  as Arc<dyn Any + Send + Sync>)
                .await;
              continue;
            }
          };
          match execute_handler(&input) {
            Ok(outcome) => {
              let _ = out_tx
                .send(Arc::new(outcome) as Arc<dyn Any + Send + Sync>)
                .await;
            }
            Err(e) => {
              let _ = err_tx.send(Arc::new(e) as Arc<dyn Any + Send + Sync>).await;
            }
          }
        }
      });

      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "error".to_string(),
        Box::pin(ReceiverStream::new(err_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
