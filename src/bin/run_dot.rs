//! CLI: Run an Attractor pipeline from a .dot file.
//!
//! Uses the compiled pipeline: parse DOT → validate → run (real exec + agent when ATTRACTOR_AGENT_CMD set).
//! Supports fix-and-retry cycles via the runner loop.
//!
//! Usage: `run_dot [OPTIONS] <path-to-dot-file>`
//! Example: run_dot examples/workflows/pre-push.dot
//!          run_dot --run-dir .attractor_run examples/workflows/pre-push.dot
//!          run_dot --resume .attractor_run examples/workflows/pre-push.dot
//!
//! Options:
//!   --run-dir DIR   Write checkpoint to DIR/checkpoint.json on successful exit.
//!   --resume DIR    Resume from checkpoint in DIR (loads DIR/checkpoint.json).
//!
//! Checkpoint is saved only at successful pipeline exit (no mid-run crash recovery).
//!
//! Set RUST_LOG=streamweave_attractor=trace for TRACE-level span enter/exit and events.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use streamweave_attractor::checkpoint_io::{self, CHECKPOINT_FILENAME};
use streamweave_attractor::{RunOptions, dot_parser, run_compiled_graph};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

fn print_usage() {
  eprintln!("Usage: run_dot [OPTIONS] <path-to-dot-file>");
  eprintln!("Options:");
  eprintln!("  --run-dir DIR   Write checkpoint to DIR/checkpoint.json on successful exit");
  eprintln!("  --resume DIR    Resume from checkpoint in DIR (loads DIR/checkpoint.json)");
  eprintln!("Example: run_dot examples/workflows/pre-push.dot");
  eprintln!("         run_dot --run-dir .attractor_run examples/workflows/pre-push.dot");
  eprintln!("         run_dot --resume .attractor_run examples/workflows/pre-push.dot");
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
    .init();

  info!("run_dot starting");
  let args: Vec<String> = env::args().collect();
  let mut run_dir: Option<PathBuf> = None;
  let mut resume_dir: Option<PathBuf> = None;
  let mut dot_path: Option<String> = None;

  let mut i = 1;
  while i < args.len() {
    match args[i].as_str() {
      "--run-dir" => {
        i += 1;
        if i >= args.len() {
          eprintln!("Error: --run-dir requires a directory");
          print_usage();
          process::exit(1);
        }
        run_dir = Some(PathBuf::from(&args[i]));
        i += 1;
      }
      "--resume" => {
        i += 1;
        if i >= args.len() {
          eprintln!("Error: --resume requires a directory");
          print_usage();
          process::exit(1);
        }
        resume_dir = Some(PathBuf::from(&args[i]));
        i += 1;
      }
      s if !s.starts_with('-') => {
        if dot_path.is_some() {
          eprintln!("Error: unexpected argument {}", s);
          print_usage();
          process::exit(1);
        }
        dot_path = Some(s.to_string());
        i += 1;
      }
      _ => {
        eprintln!("Error: unknown option {}", args[i]);
        print_usage();
        process::exit(1);
      }
    }
  }

  let path = match dot_path {
    Some(p) => p,
    None => {
      eprintln!("Error: missing path to .dot file");
      print_usage();
      process::exit(1);
    }
  };

  let resume_checkpoint = resume_dir.map(|dir| {
    let cp_path = dir.join(CHECKPOINT_FILENAME);
    match checkpoint_io::load_checkpoint(&cp_path) {
      Ok(cp) => cp,
      Err(e) => {
        eprintln!("Error loading checkpoint from {}: {}", cp_path.display(), e);
        process::exit(1);
      }
    }
  });

  let dot = match fs::read_to_string(&path) {
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

  let options = RunOptions {
    run_dir: run_dir.as_deref(),
    resume_checkpoint: resume_checkpoint.as_ref(),
  };

  let r = match run_compiled_graph(&ast, options).await {
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
