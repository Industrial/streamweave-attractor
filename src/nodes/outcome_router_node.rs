//! Routes NodeOutcome to success or fail output based on status.

use crate::types::{NodeOutcome, OutcomeStatus};
use async_trait::async_trait;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;

/// Routes NodeOutcome items: Success/PartialSuccess -> "success" port, Fail/Retry -> "fail" port.
pub struct OutcomeRouterNode {
  /// Node display name.
  name: String,
  /// Input port names.
  input_ports: Vec<String>,
  /// Output port names.
  output_ports: Vec<String>,
}

impl OutcomeRouterNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["success".to_string(), "fail".to_string()],
    }
  }
}

#[async_trait]
impl Node for OutcomeRouterNode {
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
    name == "success" || name == "fail"
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let mut in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let (success_tx, success_rx) = tokio::sync::mpsc::channel(16);
      let (fail_tx, fail_rx) = tokio::sync::mpsc::channel(16);

      tokio::spawn(async move {
        use futures::StreamExt;
        while let Some(item) = in_stream.next().await {
          if let Ok(o) = item.downcast::<NodeOutcome>() {
            let is_success = matches!(
              o.status,
              OutcomeStatus::Success | OutcomeStatus::PartialSuccess
            );
            let tx = if is_success { &success_tx } else { &fail_tx };
            let _ = tx
              .send(Arc::new(o) as Arc<dyn std::any::Any + Send + Sync>)
              .await;
          }
        }
      });

      let mut outputs = HashMap::new();
      outputs.insert(
        "success".to_string(),
        Box::pin(ReceiverStream::new(success_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn std::any::Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "fail".to_string(),
        Box::pin(ReceiverStream::new(fail_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn std::any::Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
