//! WebAssembly build pipeline.
//!
//! [`BuildWasm`] is a stateful action that compiles a package to wasm, runs
//! `wasm-bindgen`, and (in release) `wasm-opt`, reporting the output size.

mod build_wasm;

pub use build_wasm::*;
