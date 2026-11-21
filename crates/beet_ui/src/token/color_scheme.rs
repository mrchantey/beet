//! The typed, queryable color-scheme handle for non-web targets.
//!
//! On the web the active scheme is driven entirely by the
//! `.light-scheme`/`.dark-scheme` classes (seeded by `ColorSchemeScript`), and
//! CSS re-themes for free. tui/native have no CSS engine, so [`ColorScheme`] is
//! the runtime handle: set it on an ancestor element and the style plugin
//! mirrors it onto [`Classes`], re-running the cascade.
use crate::prelude::*;
use beet_core::prelude::*;

/// The active color scheme on an element, the typed counterpart to the
/// `.light-scheme`/`.dark-scheme` classes.
///
/// Query it to read the current scheme; mutate it to switch themes at runtime.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub enum ColorScheme {
	/// Dark text on a light surface.
	Light,
	/// Light text on a dark surface.
	#[default]
	Dark,
}

impl ColorScheme {
	/// Parse a scheme keyword (`light`/`dark`), eg a `color-scheme` URL or CLI
	/// parameter.
	pub fn parse(value: &str) -> Option<Self> {
		match value {
			"light" => Some(Self::Light),
			"dark" => Some(Self::Dark),
			_ => None,
		}
	}

	/// The light scheme's class.
	pub const LIGHT_CLASS: ClassName = ClassName::new_static("light-scheme");
	/// The dark scheme's class.
	pub const DARK_CLASS: ClassName = ClassName::new_static("dark-scheme");

	/// The scheme class this maps to.
	pub fn class(self) -> ClassName {
		match self {
			Self::Light => Self::LIGHT_CLASS,
			Self::Dark => Self::DARK_CLASS,
		}
	}

	/// Flip between light and dark.
	pub fn toggle(&mut self) {
		*self = match self {
			Self::Light => Self::Dark,
			Self::Dark => Self::Light,
		};
	}
}
