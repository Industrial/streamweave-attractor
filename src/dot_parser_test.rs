//! Tests for `dot_parser`.

use crate::dot_parser::{
  apply_graph_attrs, extract_edge_attrs, parse_dot, parse_identifier, parse_node_attrs,
  parse_number, parse_value, resolve_handler_from_shape, strip_comments, unescape_quoted_string,
};
use crate::types::AttractorGraph;
use std::collections::HashMap;

#[test]
fn parse_default_max_retry() {
  let dot = r#"
    digraph G {
      graph [default_max_retry=100]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let g = parse_dot(dot).unwrap();
  assert_eq!(g.default_max_retry, 100);
}

#[test]
fn parse_node_default_attrs() {
  let dot = r#"digraph G { a [type=codergen] start [shape=Mdiamond] exit [shape=Msquare] a -> start -> exit }"#;
  let g = parse_dot(dot).unwrap();
  assert!(g.nodes.get("a").unwrap().handler_type.as_deref() == Some("codergen"));
}

#[test]
fn parse_edge_with_label_condition_weight() {
  let dot = r#"
    digraph G {
      start [shape=Mdiamond]
      exit [shape=Msquare]
      start -> exit [label="ok", condition="outcome=Success", weight=10]
    }
  "#;
  let g = parse_dot(dot).unwrap();
  let e = g
    .edges
    .iter()
    .find(|e| e.from_node == "start" && e.to_node == "exit")
    .unwrap();
  assert_eq!(e.label.as_deref(), Some("ok"));
  assert_eq!(e.condition.as_deref(), Some("outcome=Success"));
  assert_eq!(e.weight, 10);
}

#[test]
fn parse_rankdir_with_semicolon() {
  let dot = r#"
    digraph G {
      rankdir=LR;
      start [shape=Mdiamond]
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let g = parse_dot(dot).unwrap();
  assert!(g.nodes.contains_key("start"));
  assert!(g.nodes.contains_key("exit"));
}

#[test]
fn parse_subgraph_skipped() {
  let dot = r#"
    digraph G {
      start [shape=Mdiamond]
      subgraph cluster_0 { x [label="inner"] }
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let g = parse_dot(dot).unwrap();
  assert!(g.nodes.contains_key("start"));
  assert!(g.nodes.contains_key("exit"));
}

#[test]
fn parse_node_goal_gate_max_retries() {
  let dot = r#"
    digraph G {
      start [shape=Mdiamond]
      run [goal_gate=true, max_retries=3]
      exit [shape=Msquare]
      start -> run -> exit
    }
  "#;
  let g = parse_dot(dot).unwrap();
  let run = g.nodes.get("run").unwrap();
  assert!(run.goal_gate);
  assert_eq!(run.max_retries, 3);
}

#[test]
fn parse_resolve_handler_shapes() {
  let dot = r#"
    digraph G {
      w [shape=hexagon]
      c [shape=diamond]
      p [shape=component]
      f [shape=tripleoctagon]
      t [shape=parallelogram]
      h [shape=house]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      start -> w -> c -> p -> f -> t -> h -> exit
    }
  "#;
  let g = parse_dot(dot).unwrap();
  assert_eq!(
    g.nodes.get("w").unwrap().handler_type.as_deref(),
    Some("wait.human")
  );
  assert_eq!(
    g.nodes.get("c").unwrap().handler_type.as_deref(),
    Some("conditional")
  );
  assert_eq!(
    g.nodes.get("p").unwrap().handler_type.as_deref(),
    Some("parallel")
  );
  assert_eq!(
    g.nodes.get("f").unwrap().handler_type.as_deref(),
    Some("parallel.fan_in")
  );
  assert_eq!(
    g.nodes.get("t").unwrap().handler_type.as_deref(),
    Some("tool")
  );
  assert_eq!(
    g.nodes.get("h").unwrap().handler_type.as_deref(),
    Some("stack.manager_loop")
  );
}

#[test]
fn parse_negative_number() {
  let dot =
    r#"digraph G { start [shape=Mdiamond] exit [shape=Msquare] a [weight=-1] start -> exit }"#;
  let g = parse_dot(dot).unwrap();
  assert!(g.nodes.contains_key("a"));
}

#[test]
fn parse_value_escape_sequences() {
  let dot = r#"
    digraph G {
      start [shape=Mdiamond, label="a\nb\tc"]
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let g = parse_dot(dot).unwrap();
  let label = g.nodes.get("start").unwrap().label.as_deref().unwrap();
  assert!(label.contains('\n'));
  assert!(label.contains('\t'));
}

#[test]
fn parse_quoted_value_escapes() {
  let dot = r#"
    digraph G {
      start [shape=Mdiamond, label="Say \"hello\""]
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let g = parse_dot(dot).unwrap();
  assert_eq!(
    g.nodes.get("start").unwrap().label.as_deref(),
    Some("Say \"hello\"")
  );
}

#[test]
fn parse_multiple_chain_edges() {
  let dot = r#"digraph G { start [shape=Mdiamond] a [] b [] exit [shape=Msquare] start -> a -> b -> exit }"#;
  let g = parse_dot(dot).unwrap();
  assert_eq!(g.edges.len(), 3);
}

#[test]
fn parse_simple_dot() {
  let dot = r#"
        digraph Simple {
            graph [goal="Run tests"]
            start [shape=Mdiamond, label="Start"]
            exit [shape=Msquare, label="Exit"]
            run [label="Run Tests"]
            start -> run -> exit
        }
    "#;
  let g = parse_dot(dot).unwrap();
  assert_eq!(g.goal, "Run tests");
  assert!(g.nodes.contains_key("start"));
  assert!(g.nodes.contains_key("exit"));
  assert!(g.nodes.contains_key("run"));
  assert_eq!(g.edges.len(), 2);
}

#[test]
fn err_no_digraph() {
  let r = parse_dot("graph foo { }");
  assert!(r.is_err());
  assert!(r.unwrap_err().contains("digraph"));
}

#[test]
fn strip_comments_removes_line_and_block() {
  let s = "a // line\nb /* block */ c";
  assert_eq!(strip_comments(s), "a \nb  c");
}

#[test]
fn parse_identifier_returns_id_and_rest() {
  let (id, rest) = parse_identifier("  foo bar").unwrap();
  assert_eq!(id, "foo");
  assert_eq!(rest, " bar");
}

#[test]
fn parse_identifier_returns_none_for_empty() {
  assert!(parse_identifier("  ;").is_none());
}

#[test]
fn parse_number_parses_positive_and_negative() {
  let (n, rest) = parse_number("42 rest").unwrap();
  assert_eq!(n, "42");
  assert_eq!(rest, " rest");
  let (n, _) = parse_number("-1]").unwrap();
  assert_eq!(n, "-1");
}

#[test]
fn unescape_quoted_string_handles_escapes() {
  assert_eq!(unescape_quoted_string("a\\nb\\tc"), "a\nb\tc");
  assert_eq!(unescape_quoted_string("Say \\\"hi\\\""), "Say \"hi\"");
}

#[test]
fn resolve_handler_from_shape_maps_shapes() {
  assert_eq!(
    resolve_handler_from_shape("Mdiamond").as_deref(),
    Some("start")
  );
  assert_eq!(
    resolve_handler_from_shape("box").as_deref(),
    Some("codergen")
  );
  assert_eq!(
    resolve_handler_from_shape("unknown").as_deref(),
    Some("codergen")
  );
}

#[test]
fn apply_graph_attrs_sets_goal_and_max_retry() {
  let mut g = AttractorGraph {
    goal: String::new(),
    nodes: HashMap::new(),
    edges: vec![],
    default_max_retry: 50,
  };
  apply_graph_attrs(
    &[
      ("goal".to_string(), "test".to_string()),
      ("default_max_retry".to_string(), "100".to_string()),
    ],
    &mut g,
  );
  assert_eq!(g.goal, "test");
  assert_eq!(g.default_max_retry, 100);
}

#[test]
fn extract_edge_attrs_gets_label_condition_weight() {
  let attrs = vec![
    ("label".to_string(), "x".to_string()),
    ("condition".to_string(), "y".to_string()),
    ("weight".to_string(), "5".to_string()),
  ];
  let (label, cond, weight) = extract_edge_attrs(&attrs);
  assert_eq!(label.as_deref(), Some("x"));
  assert_eq!(cond.as_deref(), Some("y"));
  assert_eq!(weight, 5);
}

#[test]
fn parse_value_parses_quoted_and_number() {
  let (v, rest) = parse_value("\"hello\"").unwrap();
  assert_eq!(v, "hello");
  assert!(rest.is_empty() || rest.starts_with(' '));
  let (v, _) = parse_value("123 ").unwrap();
  assert_eq!(v, "123");
}

#[test]
fn parse_node_attrs_builds_node() {
  let attrs = vec![
    ("shape".to_string(), "Mdiamond".to_string()),
    ("goal_gate".to_string(), "true".to_string()),
  ];
  let n = parse_node_attrs("start", &attrs).unwrap();
  assert_eq!(n.id, "start");
  assert_eq!(n.shape, "Mdiamond");
  assert!(n.goal_gate);
  assert_eq!(n.handler_type.as_deref(), Some("start"));
}

#[test]
fn parse_with_comments() {
  let dot = r#"
        // comment
        digraph G {
            /* block comment */
            start [shape=Mdiamond]
            exit [shape=Msquare]
            start -> exit
        }
    "#;
  let g = parse_dot(dot).unwrap();
  assert!(g.nodes.contains_key("start"));
  assert!(g.nodes.contains_key("exit"));
  assert_eq!(g.edges.len(), 1);
}
