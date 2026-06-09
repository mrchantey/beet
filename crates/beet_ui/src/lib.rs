#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
// #![deny(missing_docs)]

beet_core::test_main!();

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod document;
mod input;
mod parse;
mod render;
#[cfg(feature = "scene")]
pub mod scene;
#[cfg(feature = "style")]
pub mod style;
mod token;
mod types;
#[cfg(feature = "scene")]
mod widgets;

/// Exports the most commonly used items.
pub mod prelude {
	#[cfg(feature = "style")]
	pub use crate::canonical_property;
	#[cfg(feature = "style")]
	pub use crate::css_property;
	#[cfg(feature = "style")]
	pub use crate::css_variable;
	pub use crate::document::*;
	pub use crate::inline_class;
	pub use crate::input::*;
	pub use crate::parse::*;
	pub use crate::render::*;
	#[cfg(feature = "scene")]
	pub use crate::scene::*;
	#[cfg(feature = "style")]
	pub use crate::style;
	/// The shared class-name vocabulary, reached through the `classes::` prefix.
	#[cfg(feature = "style")]
	pub use crate::style::material::classes;
	/// The Material styling system. Its design-token roles are deliberately
	/// reached through the `material::` prefix (eg `material::colors::Primary`),
	/// since Material is one of potentially many styling systems. (Internally
	/// beet_ui's own rule definitions reach the bare `colors::` prefix via
	/// `use crate::style::material::*`.)
	#[cfg(feature = "style")]
	pub use crate::style::material;
	#[cfg(feature = "style")]
	pub use crate::style::BlinkStyle;
	#[cfg(feature = "style")]
	pub use crate::style::ColorScheme;
	#[cfg(feature = "style")]
	pub use crate::style::DecorationLine;
	#[cfg(feature = "style")]
	pub use crate::style::DecorationStyle;
	#[cfg(feature = "style")]
	pub use crate::style::FontStyle;
	#[cfg(feature = "style")]
	pub use crate::style::ResolveStylesSet;
	#[cfg(feature = "style")]
	pub use crate::style::StylePlugin;
	#[cfg(feature = "style")]
	pub use crate::style::TextAlign;
	#[cfg(feature = "style")]
	pub use crate::style::VISUAL_STYLE_DEFAULT;
	#[cfg(feature = "style")]
	pub use crate::style::VisualStyle;
	pub use crate::token;
	pub use crate::token::*;

	pub use crate::types::*;
	#[cfg(feature = "scene")]
	pub use crate::widgets::*;

	// re-exported so the `token!` macro can resolve `$crate::prelude::FieldSchema`
	pub use beet_core::prelude::FieldSchema;
}


pub mod exports {
	// used by the val! macro
	pub use beet_core::prelude::HashMap;
	#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
	pub use bevy_ratatui;
	#[cfg(all(feature = "ratatui", not(target_arch = "wasm32")))]
	pub use ratatui;
}
