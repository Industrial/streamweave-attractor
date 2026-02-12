//! StreamWeave nodes implementing Attractor pipeline logic.

mod apply_context_updates;
#[cfg(test)]
mod apply_context_updates_test;
mod check_goal_gates;
#[cfg(test)]
mod check_goal_gates_test;
mod create_checkpoint;
#[cfg(test)]
mod create_checkpoint_test;
mod execute_handler;
#[cfg(test)]
mod execute_handler_test;
mod execution_loop;
#[cfg(test)]
mod execution_loop_test;
mod find_start;
#[cfg(test)]
mod find_start_test;
mod init_context;
#[cfg(test)]
mod init_context_test;
mod parse_dot;
#[cfg(test)]
mod parse_dot_test;
mod select_edge;
#[cfg(test)]
mod select_edge_test;
mod validate_graph;
#[cfg(test)]
mod validate_graph_test;

pub use apply_context_updates::ApplyContextUpdatesNode;
pub use check_goal_gates::CheckGoalGatesNode;
pub use create_checkpoint::CreateCheckpointNode;
pub use execute_handler::ExecuteHandlerNode;
pub use execution_loop::{AttractorExecutionLoopNode, AttractorResult};
pub use find_start::FindStartNode;
pub use init_context::InitContextNode;
pub use parse_dot::ParseDotNode;
pub use select_edge::SelectEdgeNode;
pub use validate_graph::ValidateGraphNode;
