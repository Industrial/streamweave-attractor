//! Validate parsed Attractor graph (lint rules per attractor-spec §7).

use crate::types::AttractorGraph;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

/// StreamWeave node that validates an AttractorGraph.
pub struct ValidateGraphNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`, `error`).
  output_ports: Vec<String>,
}

impl ValidateGraphNode {
  /// Creates a new ValidateGraphNode with the given display name.
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string(), "error".to_string()],
    }
  }
}

/// Validates the graph (exactly one start, one exit) per attractor-spec §7.
#[instrument(level = "trace")]
pub(crate) fn validate(graph: &AttractorGraph) -> Result<(), String> {
  if graph.find_start().is_none() {
    return Err("Graph must have exactly one start node (shape=Mdiamond)".to_string());
  }
  if graph.find_exit().is_none() {
    return Err("Graph must have exactly one exit node (shape=Msquare)".to_string());
  }
  Ok(())
}

#[async_trait]
impl Node for ValidateGraphNode {
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
          let graph = match item.downcast::<AttractorGraph>() {
            Ok(arc) => (*arc).clone(),
            Err(_) => {
              let _ = err_tx
                .send(Arc::new("Expected AttractorGraph".to_string()) as Arc<dyn Any + Send + Sync>)
                .await;
              continue;
            }
          };
          match validate(&graph) {
            Ok(()) => {
              let _ = out_tx
                .send(Arc::new(graph) as Arc<dyn Any + Send + Sync>)
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
