//! Route code generation for beet applications.
//!
//! This module handles the generation of route handlers from source files,
//! building a tree structure that maps file paths to HTTP routes.
//!
//! # Structure
//!
//! Each [`RouteFileCollection`] contains multiple [`RouteFile`] entities,
//! which in turn contain [`RouteFileMethod`] entities representing individual
//! HTTP handlers.
//!
//! ```text
//! RouteFileCollection         - pages/
//! ├── RouteFile               - index.rs
//! │   ├── RouteFileMethod     - fn get() -> impl Bundle
//! │   └── RouteFileMethod     - fn post() -> Json<MyData>
//! └── RouteFile               - about.rs
//!     └── RouteFileMethod     - fn get() -> impl Bundle
//! ```

mod collect_client_action;
mod collect_client_action_group;
mod collect_combinator_route;
mod collect_route_files;
mod modify_route_file_tokens;
mod parse_route_file_md;
mod parse_route_file_rs;
mod parse_route_tree;
mod reexport_child_codegen;
mod route_codegen_plugin;
mod route_file;
mod route_file_collection;
mod route_file_method;
mod route_file_method_tree;

// Public API - types and traits needed by consumers
pub use route_file::*;
pub use route_file_collection::*;
pub use route_file_method::*;

pub use collect_client_action_group::CollectClientActions;
pub use modify_route_file_tokens::ModifyRoutePath;
pub use parse_route_tree::StaticRouteTree;

// Crate-internal systems and helpers
pub(crate) use collect_client_action::*;
pub(crate) use collect_client_action_group::*;
pub(crate) use collect_combinator_route::*;
pub(crate) use collect_route_files::*;
pub(crate) use modify_route_file_tokens::*;
pub(crate) use parse_route_file_md::*;
pub(crate) use parse_route_file_rs::*;
pub(crate) use parse_route_tree::*;
pub(crate) use reexport_child_codegen::*;
pub(crate) use route_codegen_plugin::*;
pub(crate) use route_file_method_tree::*;
