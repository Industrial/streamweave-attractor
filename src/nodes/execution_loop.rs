//! Attractor execution loop - runs the pipeline traversal until terminal.

use crate::nodes::execute_handler::{ExecuteHandlerInput, execute_handler};
use crate::nodes::select_edge::{SelectEdgeInput, select_edge};
use crate::types::{ExecutionState, NodeOutcome};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, instrument};

/// Final result of running an Attractor pipeline.
#[derive(Clone)]
pub struct AttractorResult {
  /// Outcome of the last executed node.
  pub last_outcome: NodeOutcome,
  /// Ids of all completed nodes in order.
  pub completed_nodes: Vec<String>,
  /// Final run context after the pipeline completes.
  pub context: HashMap<String, String>,
}

/// Merges outcome context_updates and status/preferred_label into the given context.
#[instrument(level = "trace", skip(context, outcome))]
pub(crate) fn apply_context_updates(context: &mut HashMap<String, String>, outcome: &NodeOutcome) {
  for (k, v) in &outcome.context_updates {
    context.insert(k.clone(), v.clone());
  }
  context.insert("outcome".to_string(), format!("{:?}", outcome.status));
  if let Some(ref l) = outcome.preferred_label {
    context.insert("preferred_label".to_string(), l.clone());
  }
}

/// Result of running the execution loop on one state.
pub(crate) enum RunLoopResult {
  /// Pipeline completed successfully.
  Ok(AttractorResult),
  /// Pipeline failed with error message.
  Err(String),
}

/// Runs the execution loop on one ExecutionState; returns result or error.
#[instrument(level = "trace", skip(state))]
pub(crate) fn run_execution_loop_once(state: &mut ExecutionState) -> RunLoopResult {
  let max_iter = 1000;
  let mut iter = 0;
  let mut last_outcome;

  loop {
    if iter >= max_iter {
      return RunLoopResult::Err("Max iterations exceeded".to_string());
    }
    iter += 1;

    info!(node_id = %state.current_node_id, iter = iter, "executing node");
    let node = match state.graph.nodes.get(&state.current_node_id) {
      Some(n) => n.clone(),
      None => {
        return RunLoopResult::Err(format!("Node not found: {}", state.current_node_id));
      }
    };

    let handler_input = ExecuteHandlerInput {
      node: node.clone(),
      context: state.context.clone(),
      graph: state.graph.clone(),
    };
    last_outcome = execute_handler(&handler_input).unwrap_or_else(crate::types::NodeOutcome::error);
    apply_context_updates(&mut state.context, &last_outcome);
    state.completed_nodes.push(state.current_node_id.clone());
    state
      .node_outcomes
      .insert(state.current_node_id.clone(), last_outcome.clone());

    let sel_input = SelectEdgeInput {
      node_id: state.current_node_id.clone(),
      outcome: last_outcome.clone(),
      context: state.context.clone(),
      graph: state.graph.clone(),
    };
    let sel_out = select_edge(&sel_input);

    match sel_out.next_node_id {
      Some(next_id) => {
        state.current_node_id = next_id;
      }
      None => {
        info!(
          completed_nodes = ?state.completed_nodes,
          "execution loop complete"
        );
        return RunLoopResult::Ok(AttractorResult {
          last_outcome: last_outcome.clone(),
          completed_nodes: state.completed_nodes.clone(),
          context: state.context.clone(),
        });
      }
    }
  }
}

/// StreamWeave node that runs the full Attractor execution loop.
pub struct AttractorExecutionLoopNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`, `error`).
  output_ports: Vec<String>,
}

impl AttractorExecutionLoopNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string(), "error".to_string()],
    }
  }
}

#[async_trait]
impl Node for AttractorExecutionLoopNode {
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
          let mut state = match item.downcast::<ExecutionState>() {
            Ok(arc) => (*arc).clone(),
            Err(_) => {
              let _ = err_tx
                .send(Arc::new("Expected ExecutionState".to_string()) as Arc<dyn Any + Send + Sync>)
                .await;
              continue;
            }
          };

          match run_execution_loop_once(&mut state) {
            RunLoopResult::Ok(result) => {
              let _ = out_tx
                .send(Arc::new(result) as Arc<dyn Any + Send + Sync>)
                .await;
            }
            RunLoopResult::Err(msg) => {
              let _ = err_tx
                .send(Arc::new(msg) as Arc<dyn Any + Send + Sync>)
                .await;
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
