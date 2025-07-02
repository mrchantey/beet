//! All about codegen for the native (usually server-side) part
//! of an application. The codegen structure in bevy is represented as a tree,
//! with the root being a [`RouteFileCollection`]. Each [`RouteFileCollection`] has an associated
//! [`CodegenFile`] which will be generated at the end of the codegen process.
//! Some [`RouteFile`] entities will also spawn a seperate [`CodegenFile`] and
//! link to it in the [`RouteFileCollection`][`CodegenFile`], ie for `.md` and `.rsx` files.
//! ```text
//! RouteFileCollection			- pages
//! ├── RouteFile 					- index.rs
//! 		├── RouteFileMethod - fn get()  -> impl Bundle
//! 		├── RouteFileMethod - fn post() -> Json<MyData>
//! ├── RouteFile
//! ```
//!
mod collect_client_action;
mod collect_client_action_group;
mod compile_router;
pub use collect_client_action::*;
pub use collect_client_action_group::*;
pub use compile_router::*;
pub use reexport_collections::*;
mod parse_route_tree;
mod reexport_collections;
mod route_file_method_tree;
pub use parse_route_tree::*;
pub use route_file_method_tree::*;
mod collect_combinator_route;
mod collect_route_files;
mod parse_route_file_md;
pub use collect_combinator_route::*;
pub use collect_route_files::*;
pub use parse_route_file_md::*;
mod parse_route_file_rs;
pub use parse_route_file_rs::*;
mod route_file;
pub use route_file::*;
mod route_file_method;
pub use route_file_method::*;
mod modify_route_file_tokens;
pub use modify_route_file_tokens::*;
mod route_file_collection;
pub use route_file_collection::*;
mod route_codegen_plugin;
pub use route_codegen_plugin::*;
