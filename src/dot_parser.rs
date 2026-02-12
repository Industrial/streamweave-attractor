//! Minimal DOT parser for Attractor pipeline graphs.
//!
//! Implements the subset defined in attractor-spec §2.

use crate::types::{AttractorEdge, AttractorGraph, AttractorNode};
use std::collections::HashMap;

/// Parse a DOT source string into an AttractorGraph.
pub fn parse_dot(source: &str) -> Result<AttractorGraph, String> {
  let source = strip_comments(source);
  let source = source.trim();

  if !source.starts_with("digraph") {
    return Err("Expected 'digraph' at start".to_string());
  }

  let rest = source["digraph".len()..].trim_start();
  let (_name, rest) = parse_identifier(rest).ok_or("Expected graph name")?;
  let rest = rest.trim_start();

  let rest = rest
    .strip_prefix('{')
    .ok_or("Expected '{' after graph name")?;

  let mut graph = AttractorGraph {
    goal: String::new(),
    nodes: HashMap::new(),
    edges: Vec::new(),
    default_max_retry: 50,
  };

  let mut remaining = rest.trim();
  while !remaining.is_empty() && !remaining.starts_with('}') {
    remaining = parse_statement(remaining, &mut graph)?;
    remaining = remaining.trim();
  }

  Ok(graph)
}

/// Strips `//` and `/* */` style comments from DOT source.
pub(crate) fn strip_comments(s: &str) -> String {
  let mut out = String::new();
  let mut i = 0;
  let bytes = s.as_bytes();
  while i < bytes.len() {
    if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
      while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
      }
      continue;
    }
    if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
      i += 2;
      while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
        i += 1;
      }
      if i + 1 < bytes.len() {
        i += 2;
      }
      continue;
    }
    out.push(bytes[i] as char);
    i += 1;
  }
  out
}

/// Parses an identifier (alphanumeric + underscore) and returns it plus the remaining string.
pub(crate) fn parse_identifier(s: &str) -> Option<(&str, &str)> {
  let s = s.trim_start();
  let start = s
    .find(|c: char| c.is_ascii_alphabetic() || c == '_')
    .unwrap_or(0);
  let end = s[start..]
    .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
    .map(|i| start + i)
    .unwrap_or(s.len());
  if start < end {
    Some((&s[start..end], &s[end..]))
  } else {
    None
  }
}

/// Parses a single graph statement and updates `graph`. Returns the unconsumed remainder.
fn parse_statement<'a>(mut s: &'a str, graph: &mut AttractorGraph) -> Result<&'a str, String> {
  s = s.trim_start();
  if s.starts_with('}') {
    return Ok(s);
  }
  if s.starts_with("graph") {
    return parse_graph_attrs(s, graph);
  }
  if s.starts_with("node") {
    return skip_attr_block(s);
  }
  if s.starts_with("edge") {
    return skip_attr_block(s);
  }
  if s.starts_with("rankdir") {
    return skip_assign(s);
  }
  if s.starts_with("subgraph") {
    return skip_subgraph(s);
  }

  let (id, rest) = parse_identifier(s).ok_or("Expected identifier")?;
  let rest = rest.trim_start();

  if rest.starts_with('[') {
    let (attrs, rest) = parse_attr_block(rest)?;
    let node = parse_node_attrs(id, &attrs)?;
    graph.nodes.insert(id.to_string(), node);
    return Ok(rest.trim_start().trim_start_matches(';'));
  }

  if rest.starts_with("->") {
    return parse_edge_stmt(id, rest, graph);
  }

  Ok(rest.trim_start().trim_start_matches(';'))
}

/// Applies graph-level attributes (goal, default_max_retry) to an AttractorGraph.
pub(crate) fn apply_graph_attrs(attrs: &[(String, String)], graph: &mut AttractorGraph) {
  for (k, v) in attrs {
    if k == "goal" {
      graph.goal = v.clone();
    } else if k == "default_max_retry" {
      graph.default_max_retry = v.parse().unwrap_or(50);
    }
  }
}

/// Parses `graph [...]` attributes (e.g. goal, default_max_retry) and updates `graph`.
fn parse_graph_attrs<'a>(mut s: &'a str, graph: &mut AttractorGraph) -> Result<&'a str, String> {
  s = s["graph".len()..].trim_start();
  let (attrs, rest) = parse_attr_block(s)?;
  apply_graph_attrs(&attrs, graph);
  Ok(rest.trim_start().trim_start_matches(';'))
}

/// List of key-value attribute pairs from DOT `[key=value,...]` blocks.
type AttrList = Vec<(String, String)>;

/// Extracts label, condition, and weight from edge attribute list.
pub(crate) fn extract_edge_attrs(
  attrs: &[(String, String)],
) -> (Option<String>, Option<String>, i32) {
  let label = attrs
    .iter()
    .find(|(k, _)| k == "label")
    .map(|(_, v)| v.clone());
  let condition = attrs
    .iter()
    .find(|(k, _)| k == "condition")
    .map(|(_, v)| v.clone());
  let weight = attrs
    .iter()
    .find(|(k, _)| k == "weight")
    .and_then(|(_, v)| v.parse().ok())
    .unwrap_or(0);
  (label, condition, weight)
}

/// Parses `[key=value,...]` and returns the attributes plus the remainder.
fn parse_attr_block(s: &str) -> Result<(AttrList, &str), String> {
  let s = s.trim_start().strip_prefix('[').ok_or("Expected '['")?;
  let mut attrs = Vec::new();
  let mut remaining = s.trim_start();
  while !remaining.starts_with(']') {
    let (k, rest) = parse_identifier(remaining).ok_or("Expected attribute key")?;
    let rest = rest.trim_start().strip_prefix('=').ok_or("Expected '='")?;
    let (v, rest) = parse_value(rest.trim_start())?;
    attrs.push((k.to_string(), v));
    remaining = rest.trim_start().trim_start_matches(',');
  }
  let rest = remaining[1..].trim_start();
  Ok((attrs, rest))
}

/// Unescapes DOT quoted string escape sequences (\\n, \\t, \\\", \\\\).
pub(crate) fn unescape_quoted_string(s: &str) -> String {
  s.replace("\\n", "\n")
    .replace("\\t", "\t")
    .replace("\\\"", "\"")
    .replace("\\\\", "\\")
}

/// Parses a quoted string, number, or identifier value and returns it plus the remainder.
pub(crate) fn parse_value(s: &str) -> Result<(String, &str), String> {
  let s = s.trim_start();
  if s.starts_with('"') {
    let mut end = 1;
    while end < s.len() {
      let c = s.as_bytes()[end];
      if c == b'\\' && end + 1 < s.len() {
        end += 2;
        continue;
      }
      if c == b'"' {
        break;
      }
      end += 1;
    }
    let v = unescape_quoted_string(&s[1..end]);
    Ok((v, s[end + 1..].trim_start()))
  } else if let Some((num, rest)) = parse_number(s) {
    Ok((num, rest))
  } else {
    let (id, rest) = parse_identifier(s).ok_or("Expected value")?;
    Ok((id.to_string(), rest))
  }
}

/// Parses an optional decimal number and returns it plus the remainder.
pub(crate) fn parse_number(s: &str) -> Option<(String, &str)> {
  let s = s.trim_start();
  let mut end = 0;
  if end < s.len() && s.as_bytes()[end] == b'-' {
    end += 1;
  }
  while end < s.len() && s.as_bytes()[end].is_ascii_digit() {
    end += 1;
  }
  if end > 0 {
    Some((s[..end].to_string(), &s[end..]))
  } else {
    None
  }
}

/// Builds an `AttractorNode` from a node id and its attribute list.
pub(crate) fn parse_node_attrs(id: &str, attrs: &[(String, String)]) -> Result<AttractorNode, String> {
  let mut shape = "box".to_string();
  let mut handler_type = None;
  let mut label = Some(id.to_string());
  let mut prompt = None;
  let mut goal_gate = false;
  let mut max_retries = 0u32;

  for (k, v) in attrs {
    match k.as_str() {
      "shape" => shape = v.clone(),
      "type" => handler_type = Some(v.clone()),
      "label" => label = Some(v.clone()),
      "prompt" => prompt = Some(v.clone()),
      "goal_gate" => goal_gate = v.eq_ignore_ascii_case("true"),
      "max_retries" => max_retries = v.parse().unwrap_or(0),
      _ => {}
    }
  }

  let handler_type = handler_type.or_else(|| resolve_handler_from_shape(&shape));

  Ok(AttractorNode {
    id: id.to_string(),
    shape,
    handler_type,
    label,
    prompt,
    goal_gate,
    max_retries,
  })
}

/// Maps DOT shape names to Attractor handler type strings.
pub(crate) fn resolve_handler_from_shape(shape: &str) -> Option<String> {
  Some(
    match shape {
      "Mdiamond" => "start",
      "Msquare" => "exit",
      "box" => "codergen",
      "hexagon" => "wait.human",
      "diamond" => "conditional",
      "component" => "parallel",
      "tripleoctagon" => "parallel.fan_in",
      "parallelogram" => "tool",
      "house" => "stack.manager_loop",
      _ => "codergen",
    }
    .to_string(),
  )
}

/// Parses an edge statement `id -> target [attrs]` and adds edges to `graph`.
fn parse_edge_stmt<'a>(
  from: &str,
  mut s: &'a str,
  graph: &mut AttractorGraph,
) -> Result<&'a str, String> {
  let mut targets = Vec::new();
  s = s["->".len()..].trim_start();
  loop {
    let (to, rest) = parse_identifier(s).ok_or("Expected target node")?;
    targets.push(to.to_string());
    let rest = rest.trim_start();
    if rest.starts_with('[') {
      let (attrs, rest) = parse_attr_block(rest)?;
      let (label, condition, weight) = extract_edge_attrs(&attrs);
      let mut prev = from;
      for t in &targets {
        graph.edges.push(AttractorEdge {
          from_node: prev.to_string(),
          to_node: t.clone(),
          label: label.clone(),
          condition: condition.clone(),
          weight,
        });
        prev = t;
      }
      return Ok(rest.trim_start().trim_start_matches(';'));
    }
    if !rest.starts_with("->") {
      let mut prev = from;
      for t in &targets {
        graph.edges.push(AttractorEdge {
          from_node: prev.to_string(),
          to_node: t.clone(),
          label: None,
          condition: None,
          weight: 0,
        });
        prev = t;
      }
      return Ok(rest);
    }
    s = rest["->".len()..].trim_start();
  }
}

/// Skips a balanced `[...]` attribute block and returns the remainder.
fn skip_attr_block(s: &str) -> Result<&str, String> {
  let s = s.trim_start();
  let idx = s.find('[').ok_or("Expected '['")?;
  let mut depth = 0;
  let _i = idx;
  for (j, c) in s[idx..].chars().enumerate() {
    match c {
      '[' => depth += 1,
      ']' => {
        depth -= 1;
        if depth == 0 {
          return Ok(&s[idx + j + 1..]);
        }
      }
      _ => {}
    }
  }
  Err("Unclosed attribute block".to_string())
}

/// Skips an assignment `key=value;` and returns the remainder.
fn skip_assign(s: &str) -> Result<&str, String> {
  let eq = s.find('=').ok_or("Expected '='")?;
  let rest = s[eq + 1..].trim_start();
  let end = rest.find(';').map(|i| i + 1).unwrap_or(rest.len());
  Ok(&s[eq + 1 + end..])
}

/// Skips a balanced `{...}` subgraph and returns the remainder.
fn skip_subgraph(s: &str) -> Result<&str, String> {
  let start = s.find('{').ok_or("Expected '{'")?;
  let mut depth = 0;
  for (i, c) in s[start..].chars().enumerate() {
    match c {
      '{' => depth += 1,
      '}' => {
        depth -= 1;
        if depth == 0 {
          return Ok(&s[start + i + 1..]);
        }
      }
      _ => {}
    }
  }
  Err("Unclosed subgraph".to_string())
}
