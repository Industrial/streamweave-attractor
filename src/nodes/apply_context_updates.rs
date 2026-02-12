//! Apply outcome context_updates to RunContext.

use crate::types::{NodeOutcome, RunContext};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;

/// Input: (context, outcome).
#[derive(Clone)]
pub struct ApplyContextUpdatesInput {
  /// The run context to be updated.
  pub context: RunContext,
  /// The node outcome whose context_updates and status are merged into context.
  pub outcome: NodeOutcome,
}

/// Processes one input item; returns RunContext if item is ApplyContextUpdatesInput.
pub(crate) fn process_apply_context_updates_item(
  item: Arc<dyn Any + Send + Sync>,
) -> Option<RunContext> {
  let input = item.downcast::<ApplyContextUpdatesInput>().ok()?;
  Some(apply_updates(&*input))
}

/// Applies outcome context_updates and status/preferred_label to context, returning the updated RunContext.
pub(crate) fn apply_updates(input: &ApplyContextUpdatesInput) -> RunContext {
  let mut ctx = input.context.clone();
  for (k, v) in &input.outcome.context_updates {
    ctx.insert(k.clone(), v.clone());
  }
  ctx.insert("outcome".to_string(), format!("{:?}", input.outcome.status));
  if let Some(ref l) = input.outcome.preferred_label {
    ctx.insert("preferred_label".to_string(), l.clone());
  }
  ctx
}

/// StreamWeave node that applies NodeOutcome.context_updates to RunContext.
pub struct ApplyContextUpdatesNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`).
  output_ports: Vec<String>,
}

impl ApplyContextUpdatesNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for ApplyContextUpdatesNode {
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
          let ctx = match process_apply_context_updates_item(item) {
            Some(c) => c,
            None => continue,
          };
          let _ = out_tx
            .send(Arc::new(ctx) as Arc<dyn Any + Send + Sync>)
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
