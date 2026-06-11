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
#[cfg(feature = "style")]
pub mod style;
mod token;
mod types;
#[cfg(feature = "template")]
mod widgets;

/// A [`World`](beet_core::prelude::World) wired with the minimal plugins required
/// to `spawn_template`: the substrate's
/// [`TemplatePlugin`](beet_core::prelude::TemplatePlugin), the
/// [`DocumentPlugin`](beet_core::prelude::DocumentPlugin) templates lean on, and
/// (when `bsx` is enabled) the default BSX event/verb vocabulary
/// ([`BsxDefaultsPlugin`](crate::prelude::BsxDefaultsPlugin)) so a parsed
/// `bx:click` resolves. Insert any required resources before spawning.
#[cfg(all(feature = "template", feature = "bsx"))]
pub fn ui_world() -> beet_core::prelude::World {
	use crate::prelude::*;
	use beet_core::prelude::*;
	(TemplatePlugin, DocumentPlugin, BsxDefaultsPlugin).into_world()
}

/// See [`ui_world`]; this variant omits the BSX vocabulary when `bsx` is off.
#[cfg(all(feature = "template", not(feature = "bsx")))]
pub fn ui_world() -> beet_core::prelude::World {
	use beet_core::prelude::*;
	(TemplatePlugin, DocumentPlugin).into_world()
}

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
	#[cfg(feature = "template")]
	pub use crate::ui_world;
	// the `rsx!` / `#[template]` snippet runtime moved to `beet_core`; re-export it
	// so the macro output and `use beet_ui::prelude::*` call sites resolve.
	#[cfg(feature = "template")]
	pub use beet_core::types::snippet::*;
	pub use crate::parse::*;
	pub use crate::render::*;
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
	pub use crate::style::AppColorScheme;
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
	#[cfg(feature = "template")]
	pub use crate::widgets::*;

	// re-exported so the `token!` macro can resolve `$crate::prelude::FieldSchema`
	pub use beet_core::prelude::FieldSchema;
}


pub mod exports {
	// used by the val! macro
	pub use beet_core::prelude::HashMap;
}
