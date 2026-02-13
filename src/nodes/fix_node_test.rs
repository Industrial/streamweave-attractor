//! Tests for `FixNode`.

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use streamweave::node::Node;
use tokio_stream::wrappers::ReceiverStream;

use super::FixNode;

#[test]
fn node_trait_methods() {
  let mut node = FixNode::new("fix");
  assert_eq!(node.name(), "fix");
  node.set_name("fix_retry");
  assert_eq!(node.name(), "fix_retry");
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
}

#[tokio::test]
async fn node_execute_forwards_one_item() {
  let node = FixNode::new("fix");
  let (tx, rx) = tokio::sync::mpsc::channel(4);
  tx.send(Arc::new(()) as Arc<dyn std::any::Any + Send + Sync>)
    .await
    .unwrap();
  drop(tx);
  let mut inputs: streamweave::node::InputStreams = HashMap::new();
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(rx))
      as std::pin::Pin<
        Box<dyn futures::Stream<Item = Arc<dyn std::any::Any + Send + Sync>> + Send>,
      >,
  );
  let mut outputs = node.execute(inputs).await.unwrap();
  let mut out = outputs.remove("out").unwrap();
  let item: Option<Arc<dyn std::any::Any + Send + Sync>> = out.next().await;
  assert!(item.is_some());
}
