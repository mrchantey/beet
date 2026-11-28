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
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

	/// The other scheme.
	pub fn other(self) -> Self {
		match self {
			Self::Light => Self::Dark,
			Self::Dark => Self::Light,
		}
	}

	/// The scheme an element resolves in by the classes of the tree: the
	/// nearest scheme class on `el` or its `ancestors` (nearest first), `None`
	/// for a tree carrying neither.
	pub fn nearest(
		el: &ElementView,
		ancestors: &[ElementView],
	) -> Option<Self> {
		core::iter::once(el).chain(ancestors).find_map(|view| {
			[Self::Light, Self::Dark].into_iter().find(|scheme| {
				view.contains_class(&scheme.class().as_selector())
			})
		})
	}

	/// Flip between light and dark.
	pub fn toggle(&mut self) {
		*self = match self {
			Self::Light => Self::Dark,
			Self::Dark => Self::Light,
		};
	}
}
