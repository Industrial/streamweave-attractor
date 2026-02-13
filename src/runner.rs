//! Compiled workflow runner: executes AttractorGraph using real exec and agent nodes.
//!
//! Handles cycles (fix-and-retry) via a Rust loop; StreamWeave graphs are DAG-only.
//! - Exec nodes: run shell command (success on exit 0)
//! - Codergen/fix nodes: invoke ATTRACTOR_AGENT_CMD (or default cursor-agent) with prompt as stdin

use crate::nodes::execution_loop::{AttractorResult, apply_context_updates};
use crate::nodes::init_context::create_initial_state;
use crate::nodes::select_edge::{SelectEdgeInput, select_edge};
use crate::types::{AttractorGraph, AttractorNode, NodeOutcome, RunContext};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::process::Command;
use tracing::{info, instrument};

/// Reads outcome.json from stage_dir (or ATTRACTOR_STAGE_DIR or .) and returns context_updates.
pub(crate) fn read_outcome_json(stage_dir: Option<&str>) -> Option<HashMap<String, String>> {
  let base = stage_dir
    .map(std::path::PathBuf::from)
    .or_else(|| {
      env::var("ATTRACTOR_STAGE_DIR")
        .ok()
        .map(std::path::PathBuf::from)
    })
    .unwrap_or_else(|| std::path::PathBuf::from("."));
  let path = base.join("outcome.json");
  if !path.exists() {
    return None;
  }
  let s = fs::read_to_string(&path).ok()?;
  let v: serde_json::Value = serde_json::from_str(&s).ok()?;
  let obj = v.get("context_updates")?.as_object()?;
  let mut map = HashMap::new();
  for (k, v) in obj {
    if let Some(s) = v.as_str() {
      map.insert(k.clone(), s.to_string());
    }
  }
  Some(map)
}

/// Executes a single node using compiled semantics (real exec, agent when ATTRACTOR_AGENT_CMD set).
#[instrument(level = "trace", skip(node, _context, _graph))]
pub fn execute_node_compiled(
  node: &AttractorNode,
  _context: &RunContext,
  _graph: &AttractorGraph,
) -> NodeOutcome {
  let handler = node.handler_type.as_deref().unwrap_or("codergen");

  match handler {
    "start" => NodeOutcome::success("Start"),
    "exit" => NodeOutcome::success("Exit"),
    "exec" => {
      let cmd = match node.command.as_ref() {
        Some(c) => c,
        None => return NodeOutcome::fail("exec node missing command"),
      };
      match Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(o) => {
          if o.status.success() {
            NodeOutcome::success("ok")
          } else {
            let msg = o
              .status
              .code()
              .map(|c| format!("exit {}", c))
              .unwrap_or_else(|| "signal".to_string());
            NodeOutcome::fail(msg)
          }
        }
        Err(e) => NodeOutcome::fail(format!("{}", e)),
      }
    }
    _ => {
      // codergen, fix, or other: invoke agent (default cursor-agent if ATTRACTOR_AGENT_CMD unset)
      let agent_cmd = env::var("ATTRACTOR_AGENT_CMD").unwrap_or_else(|_| {
        "cursor-agent --print true --output-format stream-json --stream-partial-output --model auto --force --workspace .".to_string()
      });
      let prompt = node.prompt.as_deref().unwrap_or("").to_string();
      run_agent(&agent_cmd, &prompt)
    }
  }
}

/// Runs the agent command with prompt as stdin; returns NodeOutcome based on exit code.
fn run_agent(agent_cmd: &str, prompt: &str) -> NodeOutcome {
  let parts: Vec<&str> = agent_cmd.split_whitespace().collect();
  let (bin, args) = match parts.split_first() {
    Some((b, a)) => (b, a),
    None => return NodeOutcome::fail("ATTRACTOR_AGENT_CMD is empty"),
  };

  match Command::new(bin)
    .args(args)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::inherit())
    .stderr(std::process::Stdio::inherit())
    .spawn()
  {
    Ok(mut child) => {
      if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes());
        let _ = stdin.write_all(b"\n");
      }
      match child.wait() {
        Ok(status) => {
          if status.success() {
            let mut outcome = NodeOutcome::success("agent completed");
            if let Some(updates) = read_outcome_json(None) {
              outcome.context_updates = updates;
            }
            outcome
          } else {
            let msg = status
              .code()
              .map(|c| format!("agent exit {}", c))
              .unwrap_or_else(|| "agent signal".to_string());
            NodeOutcome::fail(msg)
          }
        }
        Err(e) => NodeOutcome::fail(format!("agent wait: {}", e)),
      }
    }
    Err(e) => NodeOutcome::fail(format!("agent spawn: {}", e)),
  }
}

/// Runs the compiled workflow (real exec + agent) until exit or max iterations.
#[instrument(level = "trace", skip(ast))]
pub fn run_compiled_workflow(ast: &AttractorGraph) -> Result<AttractorResult, String> {
  let mut state = create_initial_state(ast.clone());
  let max_iter = 1000;
  let mut iter = 0;

  loop {
    if iter >= max_iter {
      return Err("Max iterations exceeded".to_string());
    }
    iter += 1;

    info!(node_id = %state.current_node_id, iter = iter, "executing node");
    let node: AttractorNode = match state.graph.nodes.get(&state.current_node_id) {
      Some(n) => n.clone(),
      None => {
        return Err(format!("Node not found: {}", state.current_node_id));
      }
    };

    let last_outcome = execute_node_compiled(&node, &state.context, &state.graph);
    apply_context_updates(&mut state.context, &last_outcome);
    state.completed_nodes.push(state.current_node_id.clone());
    state
      .node_outcomes
      .insert(state.current_node_id.clone(), last_outcome.clone());

    let sel_input = SelectEdgeInput {
      node_id: state.current_node_id.clone(),
      outcome: last_outcome.clone(),
      context: state.context.clone(),
      graph: state.graph.clone(),
    };
    let sel_out = select_edge(&sel_input);

    match sel_out.next_node_id {
      Some(next_id) => {
        state.current_node_id = next_id;
      }
      None => {
        info!(
          completed_nodes = ?state.completed_nodes,
          "compiled workflow complete"
        );
        return Ok(AttractorResult {
          last_outcome,
          completed_nodes: state.completed_nodes,
          context: state.context,
        });
      }
    }
  }
}
