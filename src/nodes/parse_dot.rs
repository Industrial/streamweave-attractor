//! Parse DOT source into AttractorGraph.

use crate::dot_parser;
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use tokio_stream::wrappers::ReceiverStream;

/// Parses DOT source into AttractorGraph.
pub(crate) fn process_dot(s: &str) -> Result<crate::types::AttractorGraph, String> {
  dot_parser::parse_dot(s)
}

/// Result of processing one parse-dot input item.
pub(crate) enum ParseDotItemResult {
  /// Parsed DOT produced an AttractorGraph.
  Graph(crate::types::AttractorGraph),
  /// Parse failed with error message.
  ParseError(String),
  /// Input was not a string.
  WrongType,
}

/// Processes one input item for ParseDotNode.
pub(crate) fn process_parse_dot_item(item: Arc<dyn Any + Send + Sync>) -> ParseDotItemResult {
  let s = match item.downcast::<String>() {
    Ok(arc) => (*arc).clone(),
    Err(_) => return ParseDotItemResult::WrongType,
  };
  match process_dot(&s) {
    Ok(g) => ParseDotItemResult::Graph(g),
    Err(e) => ParseDotItemResult::ParseError(e),
  }
}

/// StreamWeave node that parses DOT source into an AttractorGraph.
pub struct ParseDotNode {
  /// Node display name.
  name: String,
  /// Input port names (e.g. `in`).
  input_ports: Vec<String>,
  /// Output port names (e.g. `out`, `error`).
  output_ports: Vec<String>,
}

impl ParseDotNode {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      input_ports: vec!["in".to_string()],
      output_ports: vec!["out".to_string(), "error".to_string()],
    }
  }
}

#[async_trait]
impl Node for ParseDotNode {
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
    name == "out" || name == "error"
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
      let (err_tx, err_rx) = tokio::sync::mpsc::channel(16);

      tokio::spawn(async move {
        use futures::StreamExt;
        let mut s = in_stream;
        while let Some(item) = s.next().await {
          match process_parse_dot_item(item) {
            ParseDotItemResult::Graph(graph) => {
              let _ = out_tx
                .send(Arc::new(graph) as Arc<dyn Any + Send + Sync>)
                .await;
            }
            ParseDotItemResult::ParseError(e) => {
              let _ = err_tx.send(Arc::new(e) as Arc<dyn Any + Send + Sync>).await;
            }
            ParseDotItemResult::WrongType => {
              let _ = err_tx
                .send(Arc::new("Expected String".to_string()) as Arc<dyn Any + Send + Sync>)
                .await;
            }
          }
        }
      });

      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "error".to_string(),
        Box::pin(ReceiverStream::new(err_rx))
          as Pin<Box<dyn futures::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      Ok(outputs)
    })
  }
}
