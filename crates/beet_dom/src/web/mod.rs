//! Web utilities for WASM targets
mod dom_utils;
pub use self::dom_utils::*;
mod logging;
// pub use self::logging::*;
mod extensions;
// pub use self::extensions::*;

pub mod prelude {
	pub use super::dom_utils::*;
	pub use super::extensions::*;
	pub use super::logging::*;
}
