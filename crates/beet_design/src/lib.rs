#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
/// Collection of ui components
pub mod components;


/// Commonly used components for beet_design
pub mod prelude {
	pub use crate::components::*;
	pub(crate) use beet_rsx::as_beet::*;
}
