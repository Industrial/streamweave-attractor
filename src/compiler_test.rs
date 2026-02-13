//! Tests for the AttractorGraph → StreamWeave compiler.

use crate::compiler::compile_attractor_graph;
use crate::dot_parser::parse_dot;
use crate::runner::run_streamweave_graph;
use crate::types::GraphPayload;
use std::collections::HashMap;

#[test]
fn compile_rejects_exec_without_command() {
  let dot = r#"
    digraph G {
      graph [goal="test"]
      start [shape=Mdiamond]
      run [label="Run", type=exec]
      exit [shape=Msquare]
      start -> run -> exit
    }
  "#;
  let ast = parse_dot(dot).unwrap();
  match compile_attractor_graph(&ast, None) {
    Ok(_) => panic!("expected compile to fail (exec without command)"),
    Err(e) => {
      assert!(e.to_lowercase().contains("exec"));
      assert!(e.to_lowercase().contains("command"));
    }
  }
}

#[test]
fn compile_with_conditional_routing() {
  let dot = r#"
    digraph G {
      graph [goal="test"]
      start [shape=Mdiamond]
      run [type=exec, command="true"]
      fix [label="Fix"]
      exit [shape=Msquare]
      start -> run
      run -> exit [condition="outcome=success"]
      run -> fix [condition="outcome=fail"]
      fix -> run
    }
  "#;
  let ast = parse_dot(dot).unwrap();
  let graph = compile_attractor_graph(&ast, None).unwrap();
  // Graph built with direct out/error ports for run
  assert!(graph.name().contains("compiled"));
}

#[test]
fn compile_trivial_start_exit() {
  let dot = r#"
    digraph G {
      graph [goal="test"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let ast = parse_dot(dot).unwrap();
  let graph = compile_attractor_graph(&ast, None).unwrap();
  // Graph built successfully; we have start and exit as identity nodes
  assert!(graph.name().contains("compiled"));
}

#[tokio::test]
async fn run_streamweave_graph_trivial_start_exit() {
  let dot = r#"
    digraph G {
      graph [goal="test"]
      start [shape=Mdiamond]
      exit [shape=Msquare]
      start -> exit
    }
  "#;
  let ast = parse_dot(dot).unwrap();
  let graph = compile_attractor_graph(&ast, None).unwrap();
  let initial = GraphPayload::initial(HashMap::new(), "start");
  let out = run_streamweave_graph(graph, initial).await.unwrap();
  // Identity path: one trigger in → one item out
  assert!(out.is_some(), "expected one output from start→exit graph");
}

#[test]
fn compile_err_no_start() {
  let dot = r#"
    digraph G {
      graph [goal="test"]
      exit [shape=Msquare]
    }
  "#;
  let ast = parse_dot(dot).unwrap();
  match compile_attractor_graph(&ast, None) {
    Ok(_) => panic!("expected compile to fail (no start)"),
    Err(e) => assert!(e.to_lowercase().contains("start")),
  }
}

#[test]
fn compile_err_no_exit() {
  let dot = r#"
    digraph G {
      graph [goal="test"]
      start [shape=Mdiamond]
    }
  "#;
  let ast = parse_dot(dot).unwrap();
  match compile_attractor_graph(&ast, None) {
    Ok(_) => panic!("expected compile to fail (no exit)"),
    Err(e) => assert!(e.to_lowercase().contains("exit")),
  }
}

#[test]
fn compile_pre_push_dot() {
  let path =
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/workflows/pre-push.dot");
  let dot = std::fs::read_to_string(&path).unwrap();
  let ast = parse_dot(&dot).unwrap();
  let graph = compile_attractor_graph(&ast, None).unwrap();
  assert!(graph.name().contains("compiled"));
}
