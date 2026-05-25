//! WebAssembly build pipeline.
//!
//! [`BuildWasmAction`] is a route action that builds a [`BuildWasm`] config from
//! the request params, then compiles a package to wasm, runs `wasm-bindgen`, and
//! (in release) `wasm-opt`, reporting the output size.

mod build_wasm;

pub use build_wasm::*;
