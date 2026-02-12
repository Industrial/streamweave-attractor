//! Find start node ID from graph.

use crate::types::AttractorGraph;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;

/// StreamWeave node that emits the start node ID from an AttractorGraph.
pub struct FindStartNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`).
  output_ports: Vec<String>,
}

/// Returns the start node id if the graph has one.
pub(crate) fn find_start_id(graph: &AttractorGraph) -> Option<String> {
  graph.find_start().map(|n| n.id.clone())
}

/// Processes one input item; returns start id if item is a graph with a start node.
pub(crate) fn process_find_start_item(item: Arc<dyn Any + Send + Sync>) -> Option<String> {
  let graph = item.downcast::<AttractorGraph>().ok()?;
  find_start_id(&graph)
}

impl FindStartNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for FindStartNode {
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
          if let Some(id) = process_find_start_item(item) {
            let _ = out_tx
              .send(Arc::new(id) as Arc<dyn Any + Send + Sync>)
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
