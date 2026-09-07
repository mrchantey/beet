//! What a request is matched against: the route tree, the node at each of its
//! paths, and the per-request context stack.

mod action_node;
mod request_context;
/// The Rust route constructors: `route::new`, `route::exchange`, `route::fallback`.
pub mod route;
mod route_tree;
mod route_tree_builder;

pub use action_node::*;
pub use request_context::*;
pub use route_tree::*;
pub use route_tree_builder::*;
