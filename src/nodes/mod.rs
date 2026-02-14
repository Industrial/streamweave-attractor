//! StreamWeave nodes implementing Attractor pipeline logic.

mod apply_context_updates;
#[cfg(test)]
mod apply_context_updates_test;
mod check_goal_gates;
#[cfg(test)]
mod check_goal_gates_test;
mod codergen_node;
mod exec_node;
pub(crate) mod execute_handler;
#[cfg(test)]
mod execute_handler_test;
pub(crate) mod execution_loop;
#[cfg(test)]
mod execution_loop_test;
mod find_start;
#[cfg(test)]
mod find_start_test;
mod fix_node;
#[cfg(test)]
mod fix_node_test;
mod identity_node;
pub(crate) mod init_context;
#[cfg(test)]
mod init_context_test;
mod outcome_router_node;
mod parse_dot;
#[cfg(test)]
mod parse_dot_test;
pub(crate) mod select_edge;
#[cfg(test)]
mod select_edge_test;
pub(crate) mod validate_graph;
#[cfg(test)]
mod validate_graph_test;

pub use apply_context_updates::ApplyContextUpdatesNode;
pub use check_goal_gates::CheckGoalGatesNode;
pub use codergen_node::CodergenNode;
pub use exec_node::ExecNode;
pub use execution_loop::AttractorExecutionLoopNode;
pub use execution_loop::AttractorResult;
pub use find_start::FindStartNode;
pub use fix_node::FixNode;
pub use identity_node::IdentityNode;
pub use init_context::InitContextNode;
pub use outcome_router_node::OutcomeRouterNode;
pub use parse_dot::ParseDotNode;
