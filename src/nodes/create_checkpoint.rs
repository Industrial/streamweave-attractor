//! Create checkpoint from current state.

use crate::types::{Checkpoint, RunContext};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

/// Input for CreateCheckpointNode.
#[derive(Clone)]
pub struct CreateCheckpointInput {
  /// Current run context.
  pub context: RunContext,
  /// Id of the current node.
  pub current_node_id: String,
  /// Ids of completed nodes.
  pub completed_nodes: Vec<String>,
}

/// Processes one input item; returns Checkpoint if item is CreateCheckpointInput.
#[instrument(level = "trace", skip(item))]
pub(crate) fn process_create_checkpoint_item(
  item: Arc<dyn Any + Send + Sync>,
) -> Option<Checkpoint> {
  let input = item.downcast::<CreateCheckpointInput>().ok()?;
  Some(create_checkpoint_from_input(&input))
}

/// Builds a Checkpoint from CreateCheckpointInput.
#[instrument(level = "trace", skip(input))]
pub(crate) fn create_checkpoint_from_input(input: &CreateCheckpointInput) -> Checkpoint {
  Checkpoint {
    context: input.context.clone(),
    current_node_id: input.current_node_id.clone(),
    completed_nodes: input.completed_nodes.clone(),
  }
}

/// StreamWeave node that creates a Checkpoint from run state.
pub struct CreateCheckpointNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`).
  output_ports: Vec<String>,
}

impl CreateCheckpointNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for CreateCheckpointNode {
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
          if let Some(cp) = process_create_checkpoint_item(item) {
            let _ = out_tx
              .send(Arc::new(cp) as Arc<dyn Any + Send + Sync>)
              .await;
          }
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
