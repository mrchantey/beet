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
mod collect_client_actions;
mod parse_client_action;
pub use collect_client_actions::*;
pub use parse_client_action::*;
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
mod modify_file_route_tokens;
pub use modify_file_route_tokens::*;
mod file_group;
pub use file_group::*;
mod codegen_native_plugin;
pub use codegen_native_plugin::*;
mod codegen_file;
mod codegen_native_config;
pub use codegen_file::*;
pub use codegen_native_config::*;
