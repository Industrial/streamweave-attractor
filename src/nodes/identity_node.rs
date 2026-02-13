//! Identity / pass-through node for compiled graph (start and exit placeholders).
//! Forwards GraphPayload with current_node_id and completed_nodes updated to this node.

use crate::types::GraphPayload;
use async_trait::async_trait;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;
use tracing;

/// Pass-through node that forwards each input item to output unchanged.
/// Used as placeholder for start and exit nodes in Phase 1 compiled graph.
pub struct IdentityNode {
  /// Node display name.
  name: String,
  /// Input port names.
  input_ports: Vec<String>,
  /// Output port names.
  output_ports: Vec<String>,
}

impl IdentityNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for IdentityNode {
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
    let name = self.name.clone();
    Box::pin(async move {
      tracing::trace!(node = %name, "IdentityNode executing");
      let Some(mut in_stream) = inputs.remove("in") else {
        // No input connected (e.g. resume entry is another node); produce no output.
        let (out_tx, out_rx) = tokio::sync::mpsc::channel(16);
        drop(out_tx);
        let mut outputs = HashMap::new();
        outputs.insert(
          "out".to_string(),
          Box::pin(ReceiverStream::new(out_rx))
            as Pin<Box<dyn futures::Stream<Item = Arc<dyn std::any::Any + Send + Sync>> + Send>>,
        );
        return Ok(outputs);
      };
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(16);

      tokio::spawn(async move {
        use futures::StreamExt;
        while let Some(item) = in_stream.next().await {
          let out_item: Arc<dyn std::any::Any + Send + Sync> =
            if let Ok(payload) = item.clone().downcast::<GraphPayload>() {
              Arc::new(payload.with_node_completed(&name))
            } else {
              item
            };
          let _ = out_tx.send(out_item).await;
        }
      });

      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn std::any::Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
