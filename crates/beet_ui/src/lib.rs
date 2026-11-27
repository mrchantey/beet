#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]
// #![deny(missing_docs)]

beet_core::test_main!();

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod document;
mod formats;
mod input;
mod lexicon;
mod render;
#[cfg(feature = "style")]
pub mod style;
mod token;
mod types;
#[cfg(feature = "template")]
mod widgets;
/// A test/authoring [`World`](beet_core::prelude::World) wired with the minimal
/// UI plugins.
#[cfg(feature = "template")]
pub mod world_ext;

/// Exports the most commonly used items.
pub mod prelude {
	#[cfg(feature = "style")]
	pub use crate::canonical_property;
	#[cfg(feature = "style")]
	pub use crate::css_property;
	#[cfg(feature = "style")]
	pub use crate::css_variable;
	pub use crate::document::*;
	pub use crate::formats::*;
	pub use crate::inline_class;
	pub use crate::input::*;
	pub use crate::lexicon::*;
	pub use crate::render::*;
	#[cfg(feature = "style")]
	pub use crate::style;
	#[cfg(feature = "style")]
	pub use crate::style::BlinkStyle;
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
	pub use crate::style::VisualStyle;
	/// The Material styling system. Its design-token roles are deliberately
	/// reached through the `material::` prefix (eg `material::colors::Primary`),
	/// since Material is one of potentially many styling systems. (Internally
	/// beet_ui's own rule definitions reach the bare `colors::` prefix via
	/// `use crate::style::material::*`.)
	#[cfg(feature = "style")]
	pub use crate::style::material;
	#[cfg(feature = "style")]
	pub use crate::style::material::Theme;
	#[cfg(feature = "style")]
	pub use crate::style::material::ThemePalette;
	#[cfg(feature = "style")]
	pub use crate::style::material::ThemeTints;
	#[cfg(feature = "style")]
	pub use crate::style::material::ThemeTones;
	/// The shared class-name vocabulary, reached through the `classes::` prefix.
	#[cfg(feature = "style")]
	pub use crate::style::material::classes;
	pub use crate::token;
	pub use crate::token::*;
	#[cfg(feature = "template")]
	pub use crate::world_ext;
	#[cfg(feature = "template")]
	pub use beet_core::types::snippet::*;

	pub use crate::types::*;
	#[cfg(feature = "template")]
	pub use crate::widgets::*;

	// re-exported so the `token!` macro can resolve `$crate::prelude::ValueSchema`
	pub use beet_core::prelude::ValueSchema;
}

pub mod exports {
	// used by the value! macro
	pub use beet_core::prelude::HashMap;
}
