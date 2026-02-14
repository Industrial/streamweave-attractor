//! CLI: Run an Attractor pipeline from a .dot file.
//!
//! Uses the compiled pipeline: parse DOT → validate → run (real exec + agent when ATTRACTOR_AGENT_CMD set).
//! Supports fix-and-retry cycles via the runner loop.
//!
//! Usage: `run_dot [OPTIONS] <path-to-dot-file>`
//! Example: run_dot examples/workflows/pre-push.dot
//!
//! Checkpoint is written to .attractor/checkpoint.json on success.
//!
//! Set RUST_LOG=streamweave_attractor=trace for TRACE-level span enter/exit and events.

use clap::Parser;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use streamweave_attractor::{RunOptions, dot_parser, run_compiled_graph};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

const RUN_DIR: &str = ".attractor";

/// Run an Attractor pipeline from a .dot file.
///
/// Environment variables (see --help for ATTRACTOR_AGENT_CMD and ATTRACTOR_STAGE_DIR).
#[derive(Parser, Debug)]
#[command(name = "run_dot")]
#[command(
  after_help = r#"Environment variables (override --agent-cmd and --stage-dir when set):
  ATTRACTOR_AGENT_CMD   Command for agent/codergen nodes (e.g. cursor-agent). When set, agent steps
                        run this with prompt as stdin; outcome read from ATTRACTOR_STAGE_DIR.
  ATTRACTOR_STAGE_DIR   Directory for outcome.json and staging (default: .attractor).

Examples:
  run_dot examples/workflows/pre-push.dot
  run_dot --stage-dir /tmp/stage examples/workflows/pre-push.dot"#
)]
struct Args {
  /// Command for agent/codergen nodes (e.g. cursor-agent). Overridden by ATTRACTOR_AGENT_CMD if set.
  #[arg(long, value_name = "CMD")]
  agent_cmd: Option<String>,

  /// Directory for outcome.json and staging. Overridden by ATTRACTOR_STAGE_DIR if set. Default: .attractor
  #[arg(long, value_name = "DIR", default_value = RUN_DIR)]
  stage_dir: PathBuf,

  /// Path to the .dot workflow file
  #[arg(value_name = "path-to-dot-file")]
  dot_path: PathBuf,
}

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
    .init();

  info!("run_dot starting");
  let args = Args::parse();

  // Env vars override flags. These are the values used by the program (not read from env again).
  let agent_cmd = env::var("ATTRACTOR_AGENT_CMD").ok().or_else(|| args.agent_cmd.clone());
  let stage_dir = env::var("ATTRACTOR_STAGE_DIR")
    .ok()
    .map(PathBuf::from)
    .or_else(|| Some(args.stage_dir.clone()))
    .unwrap_or_else(|| PathBuf::from(RUN_DIR));
  let run_dir = PathBuf::from(RUN_DIR);

  info!(agent_cmd = ?agent_cmd, stage_dir = %stage_dir.display(), run_dir = %run_dir.display(), "options (env or flags)");

  let path = &args.dot_path;
  let dot = match fs::read_to_string(path) {
    Ok(s) => s,
    Err(e) => {
      eprintln!("Error reading {}: {}", path.display(), e);
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
    run_dir: Some(run_dir.as_path()),
    agent_cmd,
    stage_dir: Some(stage_dir),
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
