//! CLI: Run an Attractor pipeline from a .dot file.
//!
//! Usage: `run_dot PATH`
//! Example: run_dot examples/workflows/pre-push.dot

use std::env;
use std::fs;
use std::process;
use std::sync::Arc;
use streamweave_attractor::attractor_graph;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
  let args: Vec<String> = env::args().collect();
  if args.len() != 2 {
    eprintln!("Usage: run_dot <path-to-dot-file>");
    eprintln!("Example: run_dot examples/workflows/pre-push.dot");
    process::exit(1);
  }

  let path = &args[1];
  let dot = match fs::read_to_string(path) {
    Ok(s) => s,
    Err(e) => {
      eprintln!("Error reading {}: {}", path, e);
      process::exit(1);
    }
  };

  let mut graph = match attractor_graph() {
    Ok(g) => g,
    Err(e) => {
      eprintln!("Error building pipeline: {}", e);
      process::exit(1);
    }
  };

  let (input_tx, input_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn std::any::Any + Send + Sync>>(10);
  let (_error_tx, _error_rx) = mpsc::channel::<Arc<dyn std::any::Any + Send + Sync>>(10);

  if let Err(e) = graph.connect_input_channel("input", input_rx) {
    eprintln!("Error connecting input: {}", e);
    process::exit(1);
  }
  if let Err(e) = graph.connect_output_channel("output", output_tx) {
    eprintln!("Error connecting output: {}", e);
    process::exit(1);
  }
  if let Err(e) = graph.connect_output_channel("error", _error_tx) {
    eprintln!("Error connecting error: {}", e);
    process::exit(1);
  }

  if input_tx.send(Arc::new(dot)).await.is_err() {
    eprintln!("Error sending input");
    process::exit(1);
  }
  drop(input_tx);

  if let Err(e) = graph.execute().await {
    eprintln!("Pipeline execution error: {:?}", e);
    process::exit(1);
  }

  if let Some(result) = output_rx.recv().await
    && let Ok(r) = result.downcast::<streamweave_attractor::AttractorResult>()
  {
    println!("Pipeline completed.");
    println!("  Status: {:?}", r.last_outcome.status);
    println!("  Notes: {:?}", r.last_outcome.notes);
    println!("  Completed nodes: {:?}", r.completed_nodes);
    if format!("{:?}", r.last_outcome.status) != "Success" {
      process::exit(1);
    }
  }

  if let Err(e) = graph.wait_for_completion().await {
    eprintln!("Pipeline wait error: {:?}", e);
    process::exit(1);
  }
}
