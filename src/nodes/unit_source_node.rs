//! Emits a single trigger (Arc<()>) then completes. Used for start and Merge config.

use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Emits one Arc<()> then completes.
pub struct UnitSourceNode {
  name: String,
}

impl UnitSourceNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self { name: name.into() }
  }
}

#[async_trait]
impl Node for UnitSourceNode {
  fn name(&self) -> &str { &self.name }
  fn set_name(&mut self, name: &str) { self.name = name.to_string(); }
  fn input_port_names(&self) -> &[String] { &[] }
  fn output_port_names(&self) -> &[String] {
    static P: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    P.get_or_init(|| vec!["out".to_string()])
  }
  fn has_input_port(&self, _: &str) -> bool { false }
  fn has_output_port(&self, name: &str) -> bool { name == "out" }

  fn execute(
    &self,
    _inputs: InputStreams,
  ) -> Pin<Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>> {
    Box::pin(async move {
      let (tx, rx) = mpsc::channel(1);
      let _ = tx.send(Arc::new(()) as Arc<dyn Any + Send + Sync>).await;
      drop(tx);
      let mut outputs = HashMap::new();
      outputs.insert("out".to_string(), Box::pin(ReceiverStream::new(rx)) as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>);
      Ok(outputs)
    })
  }
}
