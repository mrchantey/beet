#![feature(imported_main, async_closure)]
pub use sweet::*;
#[cfg(target_arch = "wasm32")]
#[path = "./mod.rs"]
mod tests;
