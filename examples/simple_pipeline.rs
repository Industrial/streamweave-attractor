//! Run a simple Attractor pipeline via StreamWeave.

use std::sync::Arc;
use streamweave_attractor::attractor_graph;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let dot = r#"
    digraph Simple {
      graph [goal="Run tests and report"]
      rankdir=LR

      start [shape=Mdiamond, label="Start"]
      exit [shape=Msquare, label="Exit"]
      run_tests [label="Run Tests", prompt="Run the test suite"]
      report [label="Report", prompt="Summarize results"]

      start -> run_tests -> report -> exit
    }
  "#;

  let mut graph = attractor_graph()?;

  let (input_tx, input_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn std::any::Any + Send + Sync>>(10);
  let (_error_tx, _error_rx) = mpsc::channel::<Arc<dyn std::any::Any + Send + Sync>>(10);

  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", _error_tx)?;

  input_tx.send(Arc::new(dot.to_string())).await?;
  drop(input_tx);

  graph.execute().await.map_err(|e| format!("{:?}", e))?;

  if let Some(result) = output_rx.recv().await
    && let Ok(r) = result.downcast::<streamweave_attractor::AttractorResult>()
  {
    println!("Pipeline completed.");
    println!("  Status: {:?}", r.last_outcome.status);
    println!("  Notes: {:?}", r.last_outcome.notes);
    println!("  Completed nodes: {:?}", r.completed_nodes);
  }

  graph
    .wait_for_completion()
    .await
    .map_err(|e| format!("{:?}", e))?;
  Ok(())
}
