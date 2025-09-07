#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
// #![deny(missing_docs)]
#![doc = include_str!("../README.md")]
/// Color theme and utilities
pub mod color;
pub mod css;
pub mod html_elements;
/// Collection of layout components
pub mod layout;
pub mod macros;
pub mod templates;
/// Structs for use as context in components
pub mod types;
// /// Collection of mockups for all components
// // #[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "mockups")]
#[path = "codegen/mockups.rs"]
pub mod mockups;


/// Commonly used components for beet_design
pub mod prelude {
	pub use crate::color::*;
	pub use crate::css::*;
	pub use crate::csx;
	pub use crate::html_elements::*;
	pub use crate::layout::*;
	pub use crate::macros::*;
	#[cfg(feature = "mockups")]
	pub use crate::mockups::*;
	pub use crate::templates::*;
	pub use crate::types::*;
	// pub(crate) use beet_rsx::as_beet::*;
	#[allow(unused)]
	pub(crate) use beet::prelude::*;
	#[allow(unused)]
	pub(crate) mod beet {
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_core::prelude::*;
			pub use beet_rsx::prelude::*;
			pub use beet_net::prelude::*;
			pub use beet_utils::prelude::*;
			#[allow(unused)]
			pub(crate) use bevy::prelude::*;
		}
	}
}


pub mod exports {
	pub use material_colors::color::Argb;
	pub use material_colors::theme::Theme;
	pub use material_colors::theme::ThemeBuilder;
}
