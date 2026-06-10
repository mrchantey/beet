//! The typed, queryable color-scheme handle for non-web targets.
//!
//! On the web the active scheme is driven entirely by the
//! `.light-scheme`/`.dark-scheme` classes (seeded by `ColorSchemeScript`), and
//! CSS re-themes for free. tui/native have no CSS engine, so [`ColorScheme`] is
//! the runtime handle: set it on an ancestor element and [`sync_color_scheme`]
//! mirrors it onto [`Classes`], re-running the cascade via `resolve_styles`.
use crate::prelude::*;
use beet_core::prelude::*;

/// The active color scheme on an element — the typed counterpart to the
/// `.light-scheme`/`.dark-scheme` classes.
///
/// Query it to read the current scheme; mutate it to switch themes at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub enum ColorScheme {
	Light,
	Dark,
}

impl ColorScheme {
	/// The scheme class this maps to.
	pub fn class(self) -> ClassName {
		match self {
			Self::Light => classes::LIGHT_SCHEME,
			Self::Dark => classes::DARK_SCHEME,
		}
	}

	/// The opposite scheme's class, removed from [`Classes`] on sync.
	fn inactive_class(self) -> ClassName {
		match self {
			Self::Light => classes::DARK_SCHEME,
			Self::Dark => classes::LIGHT_SCHEME,
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

/// Mirror [`ColorScheme`] onto an entity's [`Classes`] whenever it changes, so
/// the class-based cascade (and thus `resolve_styles`) reflects the new scheme.
pub fn sync_color_scheme(
	mut commands: Commands,
	mut query: Query<
		(Entity, &ColorScheme, Option<&mut Classes>),
		Changed<ColorScheme>,
	>,
) {
	for (entity, scheme, classes) in query.iter_mut() {
		let active = scheme.class();
		let inactive = scheme.inactive_class();
		match classes {
			Some(mut classes) => {
				classes.remove(&inactive);
				classes.insert_class(active);
			}
			None => {
				commands.entity(entity).insert(Classes::new([active]));
			}
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use crate::style::material::MaterialStylePlugin;
	use crate::style::material::classes;

	/// Toggling [`ColorScheme`] at runtime re-resolves the cached non-web style,
	/// so the background flips between the light and dark schemes.
	#[beet_core::test]
	fn scheme_toggle_reresolves() {
		// `RealtimeParsePlugin` wires `PostParseTree` into the main loop so
		// `update_local` re-resolves styles, matching a realtime app's repaint
		let mut world =
			(MaterialStylePlugin::default(), StylePlugin, RealtimeParsePlugin)
				.into_world();
		let entity = world
			.spawn((
				rsx! { <div/> },
				Classes::new([classes::PAGE]),
				ColorScheme::Light,
			))
			.id();
		world.update_local();
		let light_bg = world
			.entity(entity)
			.get::<VisualStyle>()
			.unwrap()
			.background;

		// flip to dark at runtime — only the typed handle changes
		world
			.entity_mut(entity)
			.get_mut::<ColorScheme>()
			.unwrap()
			.toggle();
		world.update_local();
		let dark_bg = world
			.entity(entity)
			.get::<VisualStyle>()
			.unwrap()
			.background;

		light_bg.is_some().xpect_true();
		(light_bg != dark_bg).xpect_true();
		// the handle also mirrored onto the cascade classes
		world
			.entity(entity)
			.get::<Classes>()
			.unwrap()
			.contains_name(&classes::DARK_SCHEME)
			.xpect_true();
	}
}
