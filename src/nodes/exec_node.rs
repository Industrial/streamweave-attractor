//! Exec node: runs a shell command, succeeds on exit 0, fails otherwise.
//! Accepts GraphPayload (passes context), applies context_updates from outcome, emits GraphPayload.

use crate::nodes::apply_context_updates::{ApplyContextUpdatesInput, apply_updates};
use crate::types::{GraphPayload, NodeOutcome, OutcomeStatus, RunContext};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

/// Node that runs a shell command and emits NodeOutcome.
pub struct ExecNode {
  /// Node display name.
  name: String,
  /// Shell command to execute.
  command: String,
}

impl ExecNode {
  pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      command: command.into(),
    }
  }
}

#[async_trait]
impl Node for ExecNode {
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
    P.get_or_init(|| vec!["out".to_string(), "error".to_string()])
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
    let name = self.name.clone();
    let cmd = self.command.clone();
    Box::pin(async move {
      tracing::trace!(node = %name, command = %cmd, "ExecNode executing");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let (out_tx, out_rx) = mpsc::channel(16);
      let (err_tx, err_rx) = mpsc::channel(16);
      tokio::spawn(async move {
        let mut s = in_stream;
        while let Some(item) = s.next().await {
          let incoming = item.downcast::<GraphPayload>().ok();
          let context: RunContext = incoming
            .as_ref()
            .map(|p| p.context.clone())
            .unwrap_or_default();
          let (_current_node_id, completed_nodes) = incoming
            .as_ref()
            .map(|p| (p.current_node_id.clone(), p.completed_nodes.clone()))
            .unwrap_or_else(|| (String::new(), vec![]));
          let outcome = tokio::task::spawn_blocking({
            let c = cmd.clone();
            move || match Command::new("sh").arg("-c").arg(&c).output() {
              Ok(o) => {
                if o.status.success() {
                  NodeOutcome::success("ok")
                } else {
                  NodeOutcome::error(format!("exit {}", o.status.code().unwrap_or(-1)))
                }
              }
              Err(e) => NodeOutcome::error(format!("{}", e)),
            }
          })
          .await
          .unwrap_or_else(|e| NodeOutcome::error(format!("{}", e)));
          let is_success = outcome.status == OutcomeStatus::Success
            || outcome.status == OutcomeStatus::PartialSuccess;
          let updated = apply_updates(&ApplyContextUpdatesInput {
            context: context.clone(),
            outcome: outcome.clone(),
          });
          let mut completed = completed_nodes;
          completed.push(name.clone());
          let payload = GraphPayload::new(updated, Some(outcome), name.clone(), completed);
          let arc = Arc::new(payload) as Arc<dyn Any + Send + Sync>;
          let _ = if is_success {
            out_tx.send(arc).await
          } else {
            err_tx.send(arc).await
          };
        }
      });
      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "error".to_string(),
        Box::pin(ReceiverStream::new(err_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
