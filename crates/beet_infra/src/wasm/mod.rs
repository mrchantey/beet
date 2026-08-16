//! WebAssembly build pipeline.
//!
//! [`BuildWasm`] carries the build settings (markup-presettable, request
//! params applying over them) for [`BuildWasmAction`], which compiles a
//! package to wasm, runs `wasm-bindgen`, and (in release) `wasm-opt`,
//! reporting the output size.

mod build_wasm;

pub use build_wasm::*;
