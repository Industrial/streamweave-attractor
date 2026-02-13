//! Fix node: stub that receives NodeOutcome and emits trigger for retry.

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tracing;

/// Stub fix node: forwards one trigger per input (for retry loop).
pub struct FixNode {
  /// Node display name.
  name: String,
}

impl FixNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self { name: name.into() }
  }
}

#[async_trait]
impl Node for FixNode {
  fn name(&self) -> &str {
    &self.name
  }
  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }
  fn input_port_names(&self) -> &[String] {
    static P: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    P.get_or_init(|| vec!["in".to_string()])
  }
  fn output_port_names(&self) -> &[String] {
    static P: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    P.get_or_init(|| vec!["out".to_string()])
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
      tracing::trace!(node = %name, "FixNode executing");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let (tx, rx) = mpsc::channel(16);
      tokio::spawn(async move {
        let mut s = in_stream;
        while s.next().await.is_some() {
          let _ = tx.send(Arc::new(()) as Arc<dyn Any + Send + Sync>).await;
        }
      });
      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
