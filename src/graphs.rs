//! Predefined StreamWeave graphs built with the `graph!` macro.
//!
//! Use these instead of building equivalent graphs manually so that fixed
//! pipeline shapes stay declarative and easy to read.

use crate::nodes::IdentityNode;
use streamweave::graph::Graph;

/// Trivial start→exit pipeline: one identity from input to output.
///
/// Node names are `start` and `exit` to match the Attractor convention.
/// Graph I/O: `input` → start.in, exit.out → `output`.
pub fn trivial_start_exit_graph() -> Graph {
  streamweave::graph! {
    start: IdentityNode::new("start"),
    exit: IdentityNode::new("exit"),
    graph.input => start.in,
    start.out => exit.in,
    exit.out => graph.output
  }
}
