//! WebAssembly build + run pipeline.
//!
//! - [`run_wasm`] is the `cargo` runner used for `wasm32-unknown-unknown`
//!   targets (`runner = 'beet run-wasm'`): it runs `wasm-bindgen` then executes
//!   the module with the bundled Deno runner.
//! - [`build_wasm`] compiles a package to wasm, runs `wasm-bindgen`, and
//!   (in release) `wasm-opt`, reporting the output size.

mod build_wasm;
mod html_constants;
mod run_wasm;

pub use build_wasm::*;
pub use html_constants::*;
pub use run_wasm::*;
