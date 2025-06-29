//! All about codegen for the native (usually server-side) part
//! of an application. The codegen structure in bevy is represented as a tree,
//! with the root being a [`FileGroup`]. Each [`FileGroup`] has an associated
//! [`CodegenFile`] which will be generated at the end of the codegen process.
//! Some [`RouteFile`] entities will also spawn a seperate [`CodegenFile`] and
//! link to it in the [`FileGroup`][`CodegenFile`], ie for `.md` and `.rsx` files.
//! ```text
//! FileGroup 							- pages
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
pub use reexport_file_groups::*;
mod parse_route_tree;
mod reexport_file_groups;
mod route_file_method_tree;
pub use parse_route_tree::*;
pub use route_file_method_tree::*;
mod collect_combinator_route;
mod collect_file_group;
mod parse_route_file_md;
pub use collect_combinator_route::*;
pub use collect_file_group::*;
pub use parse_route_file_md::*;
mod parse_route_file_rs;
pub use parse_route_file_rs::*;
mod route_file;
pub use route_file::*;
mod route_file_method;
pub use route_file_method::*;
mod modify_route_path;
pub use modify_route_path::*;
mod file_group;
pub use file_group::*;
mod route_codegen_plugin;
pub use route_codegen_plugin::*;
mod route_codegen_root;
pub use route_codegen_root::*;
