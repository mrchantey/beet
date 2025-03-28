#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![deny(missing_docs)]
#![feature(more_qualified_paths)]
#![doc = include_str!("../README.md")]
/// Structs for use as context in components
pub mod context;
/// Collection of interactive components
pub mod interactive;
/// Collection of layout components
pub mod layout;
/// Collection of mockups for all components
#[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/mockups.rs"]
pub mod mockups;


/// Commonly used components for beet_design
pub mod prelude {
	pub use crate::context::*;
	pub use crate::interactive::*;
	pub use crate::layout::*;
	// #[cfg(not(feature = "build"))]
	// pub use crate::mockups::*;

	pub(crate) use beet_rsx::as_beet::*;
}
