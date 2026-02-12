//! Select next edge per attractor-spec §3.3.

use crate::types::{AttractorEdge, AttractorGraph, NodeOutcome, OutcomeStatus, RunContext};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;

/// Input for SelectEdgeNode.
#[derive(Clone)]
pub struct SelectEdgeInput {
  /// Current node id.
  pub node_id: String,
  /// Outcome of the current node.
  pub outcome: NodeOutcome,
  /// Current run context.
  pub context: RunContext,
  /// The attractor graph.
  pub graph: AttractorGraph,
}

/// Output: selected next node ID or None if done/fail.
#[derive(Clone)]
#[allow(dead_code)] // done field reserved for future callers
pub struct SelectEdgeOutput {
  /// Id of the next node, or None if traversal is done.
  pub next_node_id: Option<String>,
  /// Whether the pipeline is finished (reserved for future use).
  pub done: bool,
}

/// Selects the next edge per attractor-spec §3.3 (conditions, preferred_label, weights).
pub(crate) fn select_edge(input: &SelectEdgeInput) -> SelectEdgeOutput {
  let edges = input.graph.outgoing_edges(&input.node_id);
  if edges.is_empty() {
    return SelectEdgeOutput {
      next_node_id: None,
      done: input.outcome.status == OutcomeStatus::Success,
    };
  }

  let condition_matched: Vec<_> = edges
    .iter()
    .filter(|e| {
      e.condition
        .as_ref()
        .is_some_and(|c| evaluate_condition(c, &input.outcome, &input.context))
    })
    .copied()
    .collect();

  if !condition_matched.is_empty() {
    let best = best_by_weight_then_lexical(condition_matched);
    return SelectEdgeOutput {
      next_node_id: Some(best.to_node.clone()),
      done: false,
    };
  }

  if let Some(ref pref) = input.outcome.preferred_label {
    let normalized_pref = normalize_label(pref);
    for e in &edges {
      if e
        .label
        .as_ref()
        .is_some_and(|l| normalize_label(l) == normalized_pref)
      {
        return SelectEdgeOutput {
          next_node_id: Some(e.to_node.clone()),
          done: false,
        };
      }
    }
  }

  for sid in &input.outcome.suggested_next_ids {
    for e in &edges {
      if e.to_node == *sid {
        return SelectEdgeOutput {
          next_node_id: Some(e.to_node.clone()),
          done: false,
        };
      }
    }
  }

  let unconditional: Vec<_> = edges
    .iter()
    .filter(|e| e.condition.is_none())
    .copied()
    .collect();
  if !unconditional.is_empty() {
    let best = best_by_weight_then_lexical(unconditional);
    return SelectEdgeOutput {
      next_node_id: Some(best.to_node.clone()),
      done: false,
    };
  }

  let best = best_by_weight_then_lexical(edges);
  SelectEdgeOutput {
    next_node_id: Some(best.to_node.clone()),
    done: false,
  }
}

/// Evaluates a condition string (e.g. `outcome=Success`, `outcome!=Fail`) against context.
pub(crate) fn evaluate_condition(cond: &str, _outcome: &NodeOutcome, context: &RunContext) -> bool {
  let cond = cond.trim();
  if let Some(stripped) = cond.strip_prefix("outcome=") {
    let outcome_str = context.get("outcome").map(|s| s.as_str()).unwrap_or("");
    return stripped.eq_ignore_ascii_case(outcome_str)
      || (stripped.eq_ignore_ascii_case("success") && outcome_str == "SUCCESS")
      || (stripped.eq_ignore_ascii_case("fail") && outcome_str == "FAIL");
  }
  if let Some(stripped) = cond.strip_prefix("outcome!=") {
    let outcome_str = context.get("outcome").map(|s| s.as_str()).unwrap_or("");
    return !stripped.eq_ignore_ascii_case(outcome_str);
  }
  false
}

/// Normalizes an edge label for comparison (lowercase, trim, strip prefixes).
pub(crate) fn normalize_label(l: &str) -> String {
  l.to_lowercase()
    .trim()
    .trim_start_matches(|c: char| c == '[' || c.is_ascii_alphabetic())
    .trim_start_matches([')', ' ', '-'])
    .to_string()
}

/// Picks the best edge by weight (descending), then lexically by to_node.
pub(crate) fn best_by_weight_then_lexical(edges: Vec<&AttractorEdge>) -> &AttractorEdge {
  let mut v: Vec<_> = edges.into_iter().collect();
  v.sort_by(|a, b| {
    b.weight
      .cmp(&a.weight)
      .then_with(|| a.to_node.cmp(&b.to_node))
  });
  v[0]
}

/// StreamWeave node that selects the next edge per the Attractor algorithm.
pub struct SelectEdgeNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`).
  output_ports: Vec<String>,
}

impl SelectEdgeNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string()],
    }
  }
}

#[async_trait]
impl Node for SelectEdgeNode {
  fn name(&self) -> &str {
    &self.name
  }

  fn set_name(&mut self, name: &str) {
    self.name = name.to_string();
  }

  fn input_port_names(&self) -> &[String] {
    &self.input_ports
  }

  fn output_port_names(&self) -> &[String] {
    &self.output_ports
  }

  fn has_input_port(&self, name: &str) -> bool {
    name == "in"
  }

  fn has_output_port(&self, name: &str) -> bool {
    name == "out"
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    Box::pin(async move {
      let in_stream = inputs.remove("in").ok_or("Missing 'in' input")?;
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(16);

      tokio::spawn(async move {
        use futures::StreamExt;
        let mut s = in_stream;
        while let Some(item) = s.next().await {
          let input = match item.downcast::<SelectEdgeInput>() {
            Ok(arc) => (*arc).clone(),
            Err(_) => continue,
          };
          let output = select_edge(&input);
          let _ = out_tx
            .send(Arc::new(output) as Arc<dyn Any + Send + Sync>)
            .await;
        }
      });

      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
