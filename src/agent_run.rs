//! Agent invocation: run the agent command (agent_cmd option) with prompt as stdin, read outcome.json.
//! Shared by runner and CodergenNode.

use crate::types::NodeOutcome;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::process::Command;

/// Reads outcome.json from stage_dir. Returns context_updates if present.
pub(crate) fn read_outcome_json(
  stage_dir: Option<&std::path::Path>,
) -> Option<HashMap<String, String>> {
  let base = stage_dir
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| std::path::PathBuf::from(crate::DEFAULT_STAGE_DIR));
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

/// Runs the agent command with prompt as stdin; returns NodeOutcome based on exit code.
/// Used by the compiled workflow and by CodergenNode.
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
          if status.success() {
            let mut outcome = NodeOutcome::success("agent completed");
            if let Some(updates) = read_outcome_json(stage_dir) {
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
