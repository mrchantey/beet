//! Module for the build step of the router, allowing for codegen.
mod build_wasm_routes;
mod codegen_file;
mod file_group;
mod file_group_to_func_files;
mod func_file;
mod func_files_to_route_funcs;
mod route_func_tokens;
mod route_funcs_to_codegen;
mod route_funcs_to_route_tree;
pub use build_wasm_routes::*;
pub use codegen_file::*;
pub use file_group::*;
pub use file_group_to_func_files::*;
pub use func_file::*;
pub use func_files_to_route_funcs::*;
pub use route_func_tokens::*;
pub use route_funcs_to_codegen::*;
pub use route_funcs_to_route_tree::*;
