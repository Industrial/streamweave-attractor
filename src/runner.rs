//! Compiled graph runner: compile AttractorGraph to StreamWeave graph and run it.
//!
//! - [run_streamweave_graph]: run a compiled graph (one trigger in, first output out).
//! - [run_compiled_graph]: compile AST then run, return [crate::nodes::execution_loop::AttractorResult].

use crate::nodes::execution_loop::AttractorResult;
use crate::execution_log_io::{load_execution_log, resume_state_from_log, write_execution_log_partial};
use crate::nodes::execution_loop::{RunLoopResult, run_execution_loop_once};
use crate::nodes::init_context::{create_initial_state, create_initial_state_from_resume_state};
use crate::types::{AttractorGraph, ExecutionLog, GraphPayload, NodeOutcome, ResumeState};
use std::path::Path;
use std::sync::Arc;
use tracing::instrument;

/// Runs a compiled StreamWeave graph: feeds one trigger into the "input" port,
/// runs until the graph produces output on the "output" port, then returns the first output item.
///
/// The graph must have been built with `input` and `output` port names (as produced by
/// [crate::compile_attractor_graph].
#[instrument(level = "trace", skip(graph, initial))]
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

  tracing::trace!("run_streamweave_graph: calling graph.execute()");
  graph.execute().await.map_err(|e| e.to_string())?;
  tracing::trace!("run_streamweave_graph: execute done, waiting for output on rx_out.recv()");
  let first = rx_out.recv().await;
  tracing::trace!("run_streamweave_graph: received output, calling wait_for_completion()");
  graph
    .wait_for_completion()
    .await
    .map_err(|e| e.to_string())?;
  Ok(first)
}

/// Options for [run_compiled_graph].
pub struct RunOptions<'a> {
  /// If set, used to derive execution log path when [execution_log_path] is set (e.g. run_dir/execution.log.json).
  pub run_dir: Option<&'a Path>,
  /// If set, run resumes from this state (from execution log only, no checkpoint.json).
  pub resume_state: Option<ResumeState>,
  /// When true and [resume_state] is set, skip running and return already_completed (e.g. from execution log with finished_at).
  pub resume_already_completed: bool,
  /// Command for agent/codergen nodes (e.g. cursor-agent). Required if the graph has codergen nodes.
  pub agent_cmd: Option<String>,
  /// Directory for agent outcome.json and staging.
  pub stage_dir: Option<std::path::PathBuf>,
  /// If set, execution steps are recorded and written to this path as execution.log.json (on success and failure).
  pub execution_log_path: Option<std::path::PathBuf>,
}

/// Writes execution.log.json to the given path (on both success and failure).
fn write_execution_log(
  path: &Path,
  goal: &str,
  started_at: &str,
  final_status: &str,
  completed_nodes: &[String],
  steps: Vec<crate::types::ExecutionStepEntry>,
) -> Result<(), String> {
  let finished_at = chrono::Utc::now().to_rfc3339();
  let log = ExecutionLog {
    version: 1,
    goal: goal.to_string(),
    started_at: started_at.to_string(),
    finished_at: Some(finished_at),
    final_status: final_status.to_string(),
    completed_nodes: completed_nodes.to_vec(),
    steps,
  };
  let json = serde_json::to_string_pretty(&log).map_err(|e| e.to_string())?;
  std::fs::write(path, json).map_err(|e| e.to_string())?;
  Ok(())
}

/// Compiles the Attractor graph to a StreamWeave graph, runs it, and returns an [AttractorResult].
/// Uses [crate::compile_attractor_graph]. Initial context includes the graph goal.
/// When [RunOptions::execution_log_path] is set, runs via the execution loop and writes execution.log.json.
#[instrument(level = "trace", skip(ast, options))]
pub async fn run_compiled_graph(
  ast: &AttractorGraph,
  options: RunOptions<'_>,
) -> Result<AttractorResult, String> {
  if let Some(ref st) = options.resume_state {
    let exit_id = ast
      .find_exit()
      .map(|n| n.id.clone())
      .ok_or("missing exit node")?;
    let at_exit = st.current_node_id == exit_id;
    if options.resume_already_completed || (at_exit && options.execution_log_path.is_none()) {
      return Ok(AttractorResult {
        last_outcome: NodeOutcome::success("Exit"),
        completed_nodes: st.completed_nodes.clone(),
        context: st.context.clone(),
        already_completed: true,
      });
    }
  }

  if let Some(ref log_path) = options.execution_log_path {
    let exit_id = ast
      .find_exit()
      .map(|n| n.id.clone())
      .ok_or("missing exit node")?;

    let (started_at, goal, mut state) = match load_execution_log(log_path) {
      Ok(log) => {
        if let Some(from_log) = resume_state_from_log(&log, Some(exit_id.as_str())) {
          if from_log.already_completed {
            return Ok(AttractorResult {
              last_outcome: NodeOutcome::success("Exit"),
              completed_nodes: from_log.resume_state.completed_nodes.clone(),
              context: from_log.resume_state.context.clone(),
              already_completed: true,
            });
          }
          let resume_state = from_log.resume_state;
          (
            log.started_at.clone(),
            log.goal.clone(),
            create_initial_state_from_resume_state(ast.clone(), &resume_state, Some(log.steps)),
          )
        } else {
          (
            chrono::Utc::now().to_rfc3339(),
            ast.goal.clone(),
            create_initial_state(ast.clone(), Some(vec![])),
          )
        }
      }
      Err(_) => (
        chrono::Utc::now().to_rfc3339(),
        ast.goal.clone(),
        match &options.resume_state {
          Some(st) => create_initial_state_from_resume_state(ast.clone(), st, Some(vec![])),
          None => create_initial_state(ast.clone(), Some(vec![])),
        },
      ),
    };

    let mut after_step = |st: &crate::types::ExecutionState| {
      let log = ExecutionLog {
        version: 1,
        goal: goal.clone(),
        started_at: started_at.clone(),
        finished_at: None,
        final_status: "in_progress".to_string(),
        completed_nodes: st.completed_nodes.clone(),
        steps: st.step_log.clone().unwrap_or_default(),
      };
      write_execution_log_partial(log_path, &log).map_err(|e| e.to_string())
    };
    match run_execution_loop_once(&mut state, Some(&mut after_step)) {
      RunLoopResult::Ok(result) => {
        let steps = state.step_log.unwrap_or_default();
        write_execution_log(
          log_path,
          &goal,
          &started_at,
          "success",
          &result.completed_nodes,
          steps,
        )?;
        return Ok(result);
      }
      RunLoopResult::Err(e) => {
        let steps = state.step_log.unwrap_or_default();
        let completed = state.completed_nodes.clone();
        write_execution_log(log_path, &goal, &started_at, "error", &completed, steps)?;
        return Err(e);
      }
    }
  }

  let stage_dir = options
    .stage_dir
    .as_deref()
    .or_else(|| Some(std::path::Path::new(crate::DEFAULT_STAGE_DIR)));
  let entry_node_id = options
    .resume_state
    .as_ref()
    .map(|st| st.current_node_id.as_str());
  let mut graph = crate::compiler::compile_attractor_graph(
    ast,
    entry_node_id,
    options.agent_cmd.as_deref(),
    stage_dir,
  )?;
  let initial = match &options.resume_state {
    Some(st) => GraphPayload::from_resume_state(st),
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
  let (_tx_err, mut rx_err) = tokio::sync::mpsc::channel(16);

  graph
    .connect_input_channel("input", rx_in)
    .map_err(|e| e.to_string())?;
  graph
    .connect_output_channel("output", _tx_out)
    .map_err(|e| e.to_string())?;
  let has_error_port = graph.connect_output_channel("error", _tx_err).is_ok();

  tx_in
    .send(Arc::new(initial) as Arc<dyn std::any::Any + Send + Sync>)
    .await
    .map_err(|e| e.to_string())?;
  drop(tx_in);

  tracing::trace!("run_streamweave_graph: calling graph.execute()");
  graph.execute().await.map_err(|e| e.to_string())?;
  tracing::trace!("run_streamweave_graph: execute done, waiting for first of output or error");
  let first = if has_error_port {
    tokio::select! {
      Some(arc) = rx_out.recv() => Some(arc),
      Some(arc) = rx_err.recv() => Some(arc),
      else => None,
    }
  } else {
    rx_out.recv().await
  };
  // Do not wait_for_completion(); first result decides outcome, avoids hang on merge graphs.

  let payload = first
    .and_then(|arc| arc.downcast::<GraphPayload>().ok())
    .map(|p| (*p).clone());
  let (context, last_outcome, completed_nodes, _current_node_id) = payload
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

  // Run state is persisted only via execution_log_path (execution.log.json).

  Ok(AttractorResult {
    last_outcome,
    completed_nodes,
    context,
    already_completed: false,
  })
}
