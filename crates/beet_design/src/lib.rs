#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
// #![deny(missing_docs)]
#![feature(let_chains)]
#![doc = include_str!("../README.md")]
/// Color theme and utilities
pub mod color;
pub mod components;
/// Structs for use as context in components
pub mod context;
pub mod css;
pub mod html_elements;
/// Collection of interactive components
pub mod interactive;
/// Collection of layout components
pub mod layout;
pub mod macros;
// /// Collection of mockups for all components
// // #[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/mockups.rs"]
pub mod mockups;


/// Commonly used components for beet_design
pub mod prelude {
	pub use crate::mockups::*;
	pub use crate::color::*;
	pub use crate::components::*;
	pub use crate::context::*;
	pub use crate::css::*;
	pub use crate::csx;
	pub use crate::html_elements::*;
	pub use crate::interactive::*;
	pub use crate::layout::*;
	pub use crate::macros::*;
	// pub(crate) use beet_template::as_beet::*;
	#[allow(unused)]
	pub(crate) use beet::prelude::*;
	#[allow(unused)]
	pub(crate) mod beet {
		pub mod prelude {
			pub use crate::prelude::*;
			pub use beet_bevy::prelude::*;
			pub use beet_common::prelude::*;
			pub use beet_net::prelude::*;
			pub use beet_server::prelude::*;
			pub use beet_template::prelude::*;
			pub use beet_utils::prelude::*;
			pub use beet_router::prelude::*;
			#[allow(unused)]
			pub(crate) use bevy::prelude::*;
		}
		pub mod exports {
			pub use beet_server::exports::*;
		}
	}
}


pub mod exports {
	pub use material_colors::color::Argb;
	pub use material_colors::theme::Theme;
	pub use material_colors::theme::ThemeBuilder;
}
