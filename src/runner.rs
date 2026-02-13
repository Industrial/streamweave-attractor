//! Compiled graph runner: compile AttractorGraph to StreamWeave graph and run it.
//!
//! - [run_streamweave_graph]: run a compiled graph (one trigger in, first output out).
//! - [run_compiled_graph]: compile AST then run, return [crate::nodes::execution_loop::AttractorResult].

use crate::nodes::execution_loop::AttractorResult;
use crate::types::{AttractorGraph, GraphPayload, NodeOutcome};
use std::sync::Arc;

/// Runs a compiled StreamWeave graph: feeds one trigger into the "input" port,
/// runs until the graph produces output on the "output" port, then returns the first output item.
///
/// The graph must have been built with `input` and `output` port names (as produced by
/// [crate::compile_attractor_graph]).
pub async fn run_streamweave_graph(
  mut graph: streamweave::graph::Graph,
) -> Result<Option<Arc<dyn std::any::Any + Send + Sync>>, String> {
  let (tx_in, rx_in) = tokio::sync::mpsc::channel(1);
  let (_tx_out, mut rx_out) = tokio::sync::mpsc::channel(16);

  graph
    .connect_input_channel("input", rx_in)
    .map_err(|e| e.to_string())?;
  graph
    .connect_output_channel("output", _tx_out)
    .map_err(|e| e.to_string())?;

  let initial = GraphPayload::initial(std::collections::HashMap::new());
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

/// Compiles the Attractor graph to a StreamWeave graph, runs it, and returns an [AttractorResult].
/// Uses [crate::compile_attractor_graph] and [crate::run_streamweave_graph]. Initial context includes the graph goal.
pub async fn run_compiled_graph(ast: &AttractorGraph) -> Result<AttractorResult, String> {
  let mut graph = crate::compiler::compile_attractor_graph(ast)?;
  let mut ctx = std::collections::HashMap::new();
  ctx.insert("goal".to_string(), ast.goal.clone());
  ctx.insert("graph.goal".to_string(), ast.goal.clone());
  let initial = GraphPayload::initial(ctx);
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
  let (context, last_outcome) = payload
    .map(|p| {
      (
        p.context,
        p.outcome.unwrap_or_else(|| NodeOutcome::success("Exit")),
      )
    })
    .unwrap_or_else(|| {
      (
        std::collections::HashMap::new(),
        NodeOutcome::success("Exit"),
      )
    });
  Ok(AttractorResult {
    last_outcome,
    completed_nodes: vec![], // compiled graph does not track node order
    context,
  })
}
