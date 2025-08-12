//! Web utilities for WASM targets
#![allow(async_fn_in_trait)]

// we dont want rust-analyzer loading web-sys when working with native
// so cfg this entire module

mod dom_utils;
pub use self::dom_utils::*;
mod logging;
pub use self::logging::*;
mod extensions;
pub use self::extensions::*;

pub mod prelude {
		pub use super::dom_utils::*;
		pub use super::extensions::*;
		pub use super::logging::*;
}
