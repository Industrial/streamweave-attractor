//! Tests for `AttractorEdge`.

use super::AttractorEdge;

fn edge(from: &str, to: &str) -> AttractorEdge {
  AttractorEdge {
    from_node: from.to_string(),
    to_node: to.to_string(),
    label: None,
    condition: None,
    weight: 0,
  }
}

#[test]
fn construct_edge() {
  let e = edge("a", "b");
  assert_eq!(e.from_node, "a");
  assert_eq!(e.to_node, "b");
  assert!(e.label.is_none());
  assert!(e.condition.is_none());
  assert_eq!(e.weight, 0);
}

#[test]
fn edge_with_attrs() {
  let e = AttractorEdge {
    from_node: "x".to_string(),
    to_node: "y".to_string(),
    label: Some("yes".to_string()),
    condition: Some("when".to_string()),
    weight: 5,
  };
  assert_eq!(e.label.as_deref(), Some("yes"));
  assert_eq!(e.condition.as_deref(), Some("when"));
  assert_eq!(e.weight, 5);
}

#[test]
fn clone_edge() {
  let e = edge("a", "b");
  let c = e.clone();
  assert_eq!(c.from_node, e.from_node);
  assert_eq!(c.to_node, e.to_node);
}
