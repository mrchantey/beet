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
#[cfg(all(not(feature = "setup"), not(target_arch = "wasm32")))]
#[path = "codegen/mockups.rs"]
pub mod mockups;


/// Commonly used components for beet_design
pub mod prelude {
	pub use crate::context::*;
	pub use crate::interactive::*;
	pub use crate::layout::*;
	// #[cfg(not(feature = "setup"))]
	// pub use crate::mockups::*;

	pub(crate) use beet_rsx::as_beet::*;

	#[cfg(feature = "setup")]
	use beet_router::prelude::*;

	/// Gets the [`AppConfig`] for this crate
	#[cfg(feature = "setup")]
	#[rustfmt::skip]
	pub fn setup_config() -> AppConfig {
		AppConfig::new()
			.add_step(mockups_config())
	}


	/// Gets the [`GlobFileGroup`] for this crate
	#[cfg(feature = "setup")]
	pub fn mockups_config() -> BuildComponentRoutes {
		let mut mockups = BuildComponentRoutes::mockups(
			"crates/beet_design/src",
			"beet_design",
		);
		mockups.codegen_file.use_beet_tokens =
			"use beet_router::as_beet::*;".into();
		mockups
	}
}
