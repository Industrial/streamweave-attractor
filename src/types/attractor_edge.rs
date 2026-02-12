//! An edge in the Attractor DOT graph.

/// An edge in the Attractor DOT graph.
#[derive(Debug, Clone)]
pub struct AttractorEdge {
  pub from_node: String,
  pub to_node: String,
  pub label: Option<String>,
  pub condition: Option<String>,
  pub weight: i32,
}
