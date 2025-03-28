//! Module for the build step of the router, allowing for codegen.
mod build_file_route_tree;
mod build_file_routes;
mod build_wasm_routes;
mod codegen_file;
mod file_group;
mod file_group_to_funcs;
mod func_files_to_codegen;
mod func_files_to_route_funcs;
mod func_files_to_route_tree;
pub use build_file_route_tree::*;
pub use build_file_routes::*;
pub use build_wasm_routes::*;
pub use codegen_file::*;
pub use file_group::*;
pub use file_group_to_funcs::*;
pub use func_files_to_codegen::*;
pub use func_files_to_route_funcs::*;
pub use func_files_to_route_tree::*;
