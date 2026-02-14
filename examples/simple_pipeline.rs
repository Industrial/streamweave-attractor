//! Run a simple Attractor pipeline via the compiled graph.

use streamweave_attractor::{RunOptions, dot_parser, run_compiled_graph};

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

  let ast = dot_parser::parse_dot(dot)?;
  let r = run_compiled_graph(
    &ast,
    RunOptions {
      run_dir: None,
      agent_cmd: None,
      stage_dir: None,
    },
  )
  .await?;

  println!("Pipeline completed.");
  println!("  Status: {:?}", r.last_outcome.status);
  println!("  Notes: {:?}", r.last_outcome.notes);
  println!("  Completed nodes: {:?}", r.completed_nodes);
  Ok(())
}
