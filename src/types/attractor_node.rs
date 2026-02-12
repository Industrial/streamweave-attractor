//! A node in the Attractor DOT graph (parsed from DOT).

/// Returns true if the shape indicates a start node (Mdiamond).
pub(crate) fn shape_is_start(shape: &str) -> bool {
  shape.eq_ignore_ascii_case("Mdiamond")
}

/// Returns true if the shape indicates an exit node (Msquare).
pub(crate) fn shape_is_exit(shape: &str) -> bool {
  shape.eq_ignore_ascii_case("Msquare")
}

/// Returns true if the id indicates a start node.
pub(crate) fn id_is_start(id: &str) -> bool {
  id.eq_ignore_ascii_case("start")
}

/// Returns true if the id indicates an exit node.
pub(crate) fn id_is_exit(id: &str) -> bool {
  id.eq_ignore_ascii_case("exit")
}

/// A node in the Attractor DOT graph (parsed from DOT).
#[derive(Debug, Clone)]
pub struct AttractorNode {
  pub id: String,
  pub shape: String,
  pub handler_type: Option<String>,
  pub label: Option<String>,
  pub prompt: Option<String>,
  /// Command to run for `exec` handler; success on exit 0, fail otherwise.
  pub command: Option<String>,
  pub goal_gate: bool,
  pub max_retries: u32,
}

impl AttractorNode {
  pub fn is_start(&self) -> bool {
    shape_is_start(&self.shape) || id_is_start(&self.id)
  }

  pub fn is_exit(&self) -> bool {
    shape_is_exit(&self.shape) || id_is_exit(&self.id)
  }

  pub fn is_terminal(&self) -> bool {
    self.is_exit()
  }
}
