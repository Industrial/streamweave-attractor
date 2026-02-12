//! Tests for `graph`.

use crate::graph::attractor_graph;
use std::sync::Arc;
use tokio::sync::mpsc;

#[test]
fn attractor_graph_returns_ok() {
  let r = attractor_graph();
  assert!(r.is_ok());
}

#[tokio::test]
async fn pipeline_executes_end_to_end() {
  let dot = r#"
    digraph Simple {
      graph [goal="Run tests"]
      start [shape=Mdiamond, label="Start"]
      exit [shape=Msquare, label="Exit"]
      run [label="Run Tests"]
      start -> run -> exit
    }
  "#;

  let mut graph = attractor_graph().unwrap();
  let (input_tx, input_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn std::any::Any + Send + Sync>>(10);
  let (_error_tx, _error_rx) = mpsc::channel::<Arc<dyn std::any::Any + Send + Sync>>(10);

  graph.connect_input_channel("input", input_rx).unwrap();
  graph.connect_output_channel("output", output_tx).unwrap();
  graph.connect_output_channel("error", _error_tx).unwrap();

  input_tx.send(Arc::new(dot.to_string())).await.unwrap();
  drop(input_tx);

  graph.execute().await.unwrap();

  let result = output_rx.recv().await;
  assert!(result.is_some());
  if let Some(r) = result {
    let ok = r.downcast::<crate::AttractorResult>().is_ok();
    assert!(ok, "expected AttractorResult");
  }

  graph.wait_for_completion().await.unwrap();
}
