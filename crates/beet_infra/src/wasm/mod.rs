//! WebAssembly build pipeline.
//!
//! [`BuildWasm`] is a route action that parses its params from the request,
//! then compiles a package to wasm, runs `wasm-bindgen`, and (in release)
//! `wasm-opt`, reporting the output size.

mod build_wasm;

pub use build_wasm::*;
