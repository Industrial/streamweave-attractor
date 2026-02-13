//! CLI: Run an Attractor pipeline from a .dot file.
//!
//! Uses the compiled pipeline: parse DOT → validate → run (real exec + agent when ATTRACTOR_AGENT_CMD set).
//! Supports fix-and-retry cycles via the runner loop.
//!
//! Usage: `run_dot PATH`
//! Example: run_dot examples/workflows/pre-push.dot
//!
//! Set RUST_LOG=streamweave_attractor=trace for TRACE-level span enter/exit and events.

use std::env;
use std::fs;
use std::process;
use streamweave_attractor::{dot_parser, run_compiled_workflow};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
    .init();

  info!("run_dot starting");
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

  let ast = match dot_parser::parse_dot(&dot) {
    Ok(a) => a,
    Err(e) => {
      eprintln!("Error parsing DOT: {}", e);
      process::exit(1);
    }
  };

  let r = match run_compiled_workflow(&ast) {
    Ok(res) => res,
    Err(e) => {
      eprintln!("Pipeline error: {}", e);
      process::exit(1);
    }
  };

  info!(status = ?r.last_outcome.status, nodes = ?r.completed_nodes, "pipeline completed");
  println!("Pipeline completed.");
  println!("  Status: {:?}", r.last_outcome.status);
  println!("  Notes: {:?}", r.last_outcome.notes);
  println!("  Completed nodes: {:?}", r.completed_nodes);
  if format!("{:?}", r.last_outcome.status) != "Success" {
    process::exit(1);
  }
}
