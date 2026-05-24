//! Build-time code generation for the router.
//!
//! Generates Rust source from route collections: the route bundles, typed
//! `routes::` links, and server/client action callers. Gated behind the
//! `codegen` feature so the runtime router never pulls `syn`/`quote`.

mod codegen_file;
mod emit_client_actions;
mod emit_route_tree;
mod emit_routes;
mod route_codegen;
mod route_collection;
mod syn_utils;

pub use codegen_file::*;
pub use route_codegen::*;
pub use route_collection::*;

pub(crate) use emit_client_actions::*;
pub(crate) use emit_route_tree::*;
pub(crate) use emit_routes::*;
