//! Initialize run context from validated graph.

use crate::types::{AttractorGraph, ExecutionState, ExecutionStepEntry, ResumeState, RunContext};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument;

/// StreamWeave node that initializes ExecutionState from a validated graph.
pub struct InitContextNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`).
  output_ports: Vec<String>,
}

/// Processes one input item; returns ExecutionState if item is an AttractorGraph.
#[instrument(level = "trace", skip(item))]
pub(crate) fn process_init_context_item(
  item: Arc<dyn Any + Send + Sync>,
) -> Option<ExecutionState> {
  let graph = item.downcast::<AttractorGraph>().ok()?;
  Some(create_initial_state((*graph).clone(), None))
}

/// Builds ExecutionState from a validated graph.
/// When `step_log` is `Some`, the execution loop will append step entries to it.
#[instrument(level = "trace")]
pub(crate) fn create_initial_state(
  graph: AttractorGraph,
  step_log: Option<Vec<ExecutionStepEntry>>,
) -> ExecutionState {
  let start_id = graph
    .find_start()
    .map(|n| n.id.clone())
    .unwrap_or_else(|| "start".to_string());
  let mut context: RunContext = HashMap::new();
  context.insert("goal".to_string(), graph.goal.clone());
  context.insert("graph.goal".to_string(), graph.goal.clone());
  ExecutionState {
    graph: graph.clone(),
    context,
    current_node_id: start_id,
    completed_nodes: vec![],
    node_outcomes: HashMap::new(),
    step_log,
  }
}

/// Builds ExecutionState from resume state (execution log only, no checkpoint.json).
#[instrument(level = "trace", skip(st))]
pub(crate) fn create_initial_state_from_resume_state(
  graph: AttractorGraph,
  st: &ResumeState,
  step_log: Option<Vec<ExecutionStepEntry>>,
) -> ExecutionState {
  ExecutionState {
    graph,
    context: st.context.clone(),
    current_node_id: st.current_node_id.clone(),
    completed_nodes: st.completed_nodes.clone(),
    node_outcomes: HashMap::new(),
    step_log,
  }
}

impl InitContextNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for InitContextNode {
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
          if let Some(state) = process_init_context_item(item) {
            let _ = out_tx
              .send(Arc::new(state) as Arc<dyn Any + Send + Sync>)
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
