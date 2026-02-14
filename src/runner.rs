//! Compiled graph runner: compile AttractorGraph to StreamWeave graph and run it.
//!
//! - [run_streamweave_graph]: run a compiled graph (one trigger in, first output out).
//! - [run_compiled_graph]: compile AST then run, return [crate::nodes::execution_loop::AttractorResult].

use crate::checkpoint_io::{self, CHECKPOINT_FILENAME};
use crate::nodes::execution_loop::AttractorResult;
use crate::types::{AttractorGraph, Checkpoint, GraphPayload, NodeOutcome};
use std::path::Path;
use std::sync::Arc;

/// Runs a compiled StreamWeave graph: feeds one trigger into the "input" port,
/// runs until the graph produces output on the "output" port, then returns the first output item.
///
/// The graph must have been built with `input` and `output` port names (as produced by
/// [crate::compile_attractor_graph].
pub async fn run_streamweave_graph(
  mut graph: streamweave::graph::Graph,
  initial: GraphPayload,
) -> Result<Option<Arc<dyn std::any::Any + Send + Sync>>, String> {
  let (tx_in, rx_in) = tokio::sync::mpsc::channel(1);
  let (_tx_out, mut rx_out) = tokio::sync::mpsc::channel(16);

  graph
    .connect_input_channel("input", rx_in)
    .map_err(|e| e.to_string())?;
  graph
    .connect_output_channel("output", _tx_out)
    .map_err(|e| e.to_string())?;

  tx_in
    .send(Arc::new(initial) as Arc<dyn std::any::Any + Send + Sync>)
    .await
    .map_err(|e| e.to_string())?;
  drop(tx_in);

  graph.execute().await.map_err(|e| e.to_string())?;
  let first = rx_out.recv().await;
  graph
    .wait_for_completion()
    .await
    .map_err(|e| e.to_string())?;
  Ok(first)
}

/// Options for [run_compiled_graph].
pub struct RunOptions<'a> {
  /// If set, checkpoint is written here at successful exit (to `run_dir/checkpoint.json`).
  pub run_dir: Option<&'a Path>,
  /// If set, resume from this checkpoint (entry node and initial payload from checkpoint).
  pub resume_checkpoint: Option<&'a Checkpoint>,
  /// Command for agent/codergen nodes (e.g. cursor-agent). Required if the graph has codergen nodes.
  pub agent_cmd: Option<String>,
  /// Directory for agent outcome.json and staging.
  pub stage_dir: Option<std::path::PathBuf>,
}

/// Compiles the Attractor graph to a StreamWeave graph, runs it, and returns an [AttractorResult].
/// Uses [crate::compile_attractor_graph]. Initial context includes the graph goal unless resuming.
pub async fn run_compiled_graph(
  ast: &AttractorGraph,
  options: RunOptions<'_>,
) -> Result<AttractorResult, String> {
  let entry_node_id = options
    .resume_checkpoint
    .map(|cp| cp.current_node_id.as_str());
  let mut graph = crate::compiler::compile_attractor_graph(
    ast,
    entry_node_id,
    options.agent_cmd.as_deref(),
    options.stage_dir.as_deref(),
  )?;

  let initial = match options.resume_checkpoint {
    Some(cp) => GraphPayload::from_checkpoint(cp),
    None => {
      let mut ctx = std::collections::HashMap::new();
      ctx.insert("goal".to_string(), ast.goal.clone());
      ctx.insert("graph.goal".to_string(), ast.goal.clone());
      let start_id = ast
        .find_start()
        .map(|n| n.id.clone())
        .ok_or("missing start node")?;
      GraphPayload::initial(ctx, start_id)
    }
  };

  let (tx_in, rx_in) = tokio::sync::mpsc::channel(1);
  let (_tx_out, mut rx_out) = tokio::sync::mpsc::channel(16);

  graph
    .connect_input_channel("input", rx_in)
    .map_err(|e| e.to_string())?;
  graph
    .connect_output_channel("output", _tx_out)
    .map_err(|e| e.to_string())?;

  tx_in
    .send(Arc::new(initial) as Arc<dyn std::any::Any + Send + Sync>)
    .await
    .map_err(|e| e.to_string())?;
  drop(tx_in);

  graph.execute().await.map_err(|e| e.to_string())?;
  let first = rx_out.recv().await;
  graph
    .wait_for_completion()
    .await
    .map_err(|e| e.to_string())?;

  let payload = first
    .and_then(|arc| arc.downcast::<GraphPayload>().ok())
    .map(|p| (*p).clone());
  let (context, last_outcome, completed_nodes, current_node_id) = payload
    .as_ref()
    .map(|p| {
      (
        p.context.clone(),
        p.outcome
          .clone()
          .unwrap_or_else(|| NodeOutcome::success("Exit")),
        p.completed_nodes.clone(),
        p.current_node_id.clone(),
      )
    })
    .unwrap_or_else(|| {
      (
        std::collections::HashMap::new(),
        NodeOutcome::success("Exit"),
        vec![],
        String::new(),
      )
    });

  if let Some(run_dir) = options.run_dir {
    let cp = Checkpoint {
      context: context.clone(),
      current_node_id: current_node_id.clone(),
      completed_nodes: completed_nodes.clone(),
    };
    let path = run_dir.join(CHECKPOINT_FILENAME);
    checkpoint_io::save_checkpoint(&path, &cp).map_err(|e| e.to_string())?;
  }

  Ok(AttractorResult {
    last_outcome,
    completed_nodes,
    context,
  })
}
