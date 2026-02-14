//! Codergen node: runs the configured agent command with prompt as stdin,
//! emits GraphPayload with NodeOutcome and updated context (context_updates applied).

use crate::agent_run;
use crate::nodes::apply_context_updates::{ApplyContextUpdatesInput, apply_updates};
use crate::types::{GraphPayload, NodeOutcome, OutcomeStatus, RunContext};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

/// Node that runs the agent command with prompt as stdin and emits NodeOutcome.
pub struct CodergenNode {
  /// Node display name.
  name: String,
  /// Prompt sent to the agent as stdin.
  prompt: String,
  /// Agent command (e.g. cursor-agent). Required.
  agent_cmd: Option<String>,
  /// Stage directory for outcome.json.
  stage_dir: Option<std::path::PathBuf>,
}

impl CodergenNode {
  pub fn new(
    name: impl Into<String>,
    prompt: impl Into<String>,
    agent_cmd: Option<String>,
    stage_dir: Option<std::path::PathBuf>,
  ) -> Self {
    Self {
      name: name.into(),
      prompt: prompt.into(),
      agent_cmd,
      stage_dir,
    }
  }
}

#[async_trait]
impl Node for CodergenNode {
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
    let prompt = self.prompt.clone();
    let agent_cmd = self.agent_cmd.clone();
    let stage_dir = self.stage_dir.clone();
    Box::pin(async move {
      tracing::trace!(node = %name, "CodergenNode executing");
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let (out_tx, out_rx) = mpsc::channel(16);
      let (err_tx, err_rx) = mpsc::channel(16);
      tokio::spawn(async move {
        let mut s = in_stream;
        while let Some(item) = s.next().await {
          tracing::trace!(node = %name, "CodergenNode received item, processing");
          tracing::info!(node = %name, "running");
          let incoming = item.downcast::<GraphPayload>().ok();
          let context: RunContext = incoming
            .as_ref()
            .map(|p| p.context.clone())
            .unwrap_or_default();
          let (_current_node_id, completed_nodes) = incoming
            .as_ref()
            .map(|p| (p.current_node_id.clone(), p.completed_nodes.clone()))
            .unwrap_or_else(|| (String::new(), vec![]));
          let outcome = match &agent_cmd {
            None => {
              tracing::error!(node = %name, "attractor agent cmd is not set");
              NodeOutcome::error("ATTRACTOR_AGENT_CMD (or --agent-cmd) is not set")
            }
            Some(c) if c.is_empty() => {
              tracing::error!(node = %name, "attractor agent cmd is not set");
              NodeOutcome::error("ATTRACTOR_AGENT_CMD (or --agent-cmd) is not set")
            }
            Some(cmd) => {
              let cmd = cmd.clone();
              let p = prompt.clone();
              let dir = stage_dir.clone();
              tokio::task::spawn_blocking(move || agent_run::run_agent(&cmd, &p, dir.as_deref()))
                .await
                .unwrap_or_else(|e| NodeOutcome::error(format!("{}", e)))
            }
          };
          let is_success = outcome.status == OutcomeStatus::Success
            || outcome.status == OutcomeStatus::PartialSuccess;
          let updated = apply_updates(&ApplyContextUpdatesInput {
            context: context.clone(),
            outcome: outcome.clone(),
          });
          let mut completed: Vec<String> = completed_nodes;
          completed.push(name.clone());
          let payload = GraphPayload::new(updated, Some(outcome), name.clone(), completed);
          let arc = Arc::new(payload) as Arc<dyn Any + Send + Sync>;
          // Process all items (while let); cyclic graphs (e.g. beads-worker-loop) feed multiple's stream ends,
          // and used so the downstream sees stream close and wait_for_completion() can finish.
          
          if is_success {
            tracing::trace!(node = %name, "CodergenNode sending to out port");
            tracing::info!(node = %name, "finished: success");
            let _ = out_tx.send(arc).await;
          } else {
            tracing::trace!(node = %name, "CodergenNode sending to error port");
            tracing::info!(node = %name, "finished: error");
            let _ = err_tx.send(arc).await;
          }
        }
        drop(out_tx);
        drop(err_tx);
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
