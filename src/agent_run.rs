//! Agent invocation: run the agent command (agent_cmd option) with prompt as stdin, read outcome.json.
//! Shared by runner and CodergenNode.

use crate::types::NodeOutcome;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tracing::instrument;

/// Resolves the stage directory path (defaults to DEFAULT_STAGE_DIR when None).
fn stage_path(stage_dir: Option<&Path>) -> std::path::PathBuf {
  stage_dir
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::path::PathBuf::from(crate::DEFAULT_STAGE_DIR))
}

/// Reads outcome.json and returns (outcome from file if present, context_updates).
/// outcome: None = no file or no "outcome" field; Some(true) = success; Some(false) = fail/error.
pub(crate) fn read_outcome_file(
  stage_dir: Option<&Path>,
) -> Option<(Option<bool>, HashMap<String, String>)> {
  let path = stage_path(stage_dir).join("outcome.json");
  if !path.exists() {
    return None;
  }
  let s = fs::read_to_string(&path).ok()?;
  let v: serde_json::Value = serde_json::from_str(&s).ok()?;
  let outcome_str = v.get("outcome").and_then(|o| o.as_str());
  let is_success = outcome_str.map(|s| {
    let lower = s.trim().to_lowercase();
    !(lower == "fail" || lower == "error")
  });
  let empty: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
  let obj = v
    .get("context_updates")
    .and_then(|o| o.as_object())
    .unwrap_or(&empty);
  let mut map = HashMap::new();
  for (k, v) in obj {
    if let Some(s) = v.as_str() {
      map.insert(k.clone(), s.to_string());
    }
  }
  Some((is_success, map))
}

/// Runs the agent command with prompt as stdin; returns NodeOutcome based on exit code.
/// Used by the compiled workflow and by CodergenNode.
#[instrument(level = "trace", skip(agent_cmd, prompt, stage_dir))]
pub(crate) fn run_agent(
  agent_cmd: &str,
  prompt: &str,
  stage_dir: Option<&std::path::Path>,
) -> NodeOutcome {
  let parts: Vec<&str> = agent_cmd.split_whitespace().collect();
  let (bin, args) = match parts.split_first() {
    Some((b, a)) => (b, a),
    None => return NodeOutcome::error("agent_cmd is empty"),
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
          let from_file = read_outcome_file(stage_dir);
          let use_file_fail =
            status.success() && from_file.as_ref().is_some_and(|(o, _)| o == &Some(false));
          if use_file_fail {
            let (_, updates) = from_file.unwrap();
            let mut outcome = NodeOutcome::error("agent reported outcome=fail in outcome.json");
            outcome.context_updates = updates;
            outcome
          } else if status.success() {
            let mut outcome = NodeOutcome::success("agent completed");
            if let Some((_, updates)) = from_file {
              outcome.context_updates = updates;
            }
            outcome
          } else {
            let msg = status
              .code()
              .map(|c| format!("agent exit {}", c))
              .unwrap_or_else(|| "agent signal".to_string());
            NodeOutcome::error(msg)
          }
        }
        Err(e) => NodeOutcome::error(format!("agent wait: {}", e)),
      }
    }
    Err(e) => NodeOutcome::error(format!("agent spawn: {}", e)),
  }
}
