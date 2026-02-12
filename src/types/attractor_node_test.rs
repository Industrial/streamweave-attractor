//! Tests for `AttractorNode`.

use super::attractor_node::{
  id_is_exit, id_is_start, shape_is_exit, shape_is_start, AttractorNode,
};

fn node(id: &str, shape: &str) -> AttractorNode {
  AttractorNode {
    id: id.to_string(),
    shape: shape.to_string(),
    handler_type: None,
    label: None,
    prompt: None,
    goal_gate: false,
    max_retries: 0,
  }
}

#[test]
fn shape_is_start_and_id_helpers() {
  assert!(shape_is_start("Mdiamond"));
  assert!(shape_is_start("mdiamond"));
  assert!(!shape_is_start("box"));
  assert!(id_is_start("start"));
  assert!(id_is_start("START"));
  assert!(shape_is_exit("Msquare"));
  assert!(id_is_exit("exit"));
}

#[test]
fn is_start_by_shape_mdiamond() {
  let n = node("foo", "Mdiamond");
  assert!(n.is_start());
}

#[test]
fn is_start_by_shape_mdiamond_lowercase() {
  let n = node("foo", "mdiamond");
  assert!(n.is_start());
}

#[test]
fn is_start_by_id_start() {
  let n = node("start", "ellipse");
  assert!(n.is_start());
}

#[test]
fn is_start_by_id_start_uppercase() {
  let n = node("START", "ellipse");
  assert!(n.is_start());
}

#[test]
fn is_start_neither() {
  let n = node("foo", "ellipse");
  assert!(!n.is_start());
}

#[test]
fn is_exit_by_shape_msquare() {
  let n = node("foo", "Msquare");
  assert!(n.is_exit());
}

#[test]
fn is_exit_by_shape_msquare_lowercase() {
  let n = node("foo", "msquare");
  assert!(n.is_exit());
}

#[test]
fn is_exit_by_id_exit() {
  let n = node("exit", "ellipse");
  assert!(n.is_exit());
}

#[test]
fn is_exit_by_id_exit_uppercase() {
  let n = node("EXIT", "ellipse");
  assert!(n.is_exit());
}

#[test]
fn is_exit_neither() {
  let n = node("foo", "ellipse");
  assert!(!n.is_exit());
}

#[test]
fn is_terminal_when_exit() {
  let n = node("exit", "ellipse");
  assert!(n.is_terminal());
}

#[test]
fn is_terminal_when_not_exit() {
  let n = node("foo", "ellipse");
  assert!(!n.is_terminal());
}
