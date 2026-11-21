//! [`sync_color_scheme`], mirroring a [`ColorScheme`] onto its element's
//! [`Classes`] so the cascade re-themes.
use crate::prelude::*;
use beet_core::prelude::*;

impl ColorScheme {
	/// The opposite scheme's class, removed from [`Classes`] on sync.
	fn inactive_class(self) -> ClassName {
		match self {
			Self::Light => Self::DARK_CLASS,
			Self::Dark => Self::LIGHT_CLASS,
		}
	}
}

/// Mirror [`ColorScheme`] onto an entity's [`Classes`] whenever it changes, so
/// the class-based cascade (and thus `resolve_styles`) reflects the new scheme.
pub(crate) fn sync_color_scheme(
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
	/// so the surface foreground flips between the light and dark schemes. The
	/// page's surface *fill* is web-only, so the foreground (not the background)
	/// is what the terminal cascade themes per scheme.
	#[beet_core::test]
	fn scheme_toggle_reresolves() {
		// `RealtimeParsePlugin` wires `PostParseTree` into the main loop so
		// `update_local` re-resolves styles, matching a realtime app's repaint
		let mut world = (
			MaterialStylePlugin::default(),
			StylePlugin,
			RealtimeParsePlugin,
		)
			.into_world();
		let entity = world
			.spawn((
				rsx! { <div/> },
				Classes::new([classes::PAGE]),
				ColorScheme::Light,
			))
			.id();
		world.update_local();
		let light_fg = world
			.entity(entity)
			.get::<VisualStyle>()
			.unwrap()
			.foreground;

		// flip to dark at runtime — only the typed handle changes
		world
			.entity_mut(entity)
			.get_mut::<ColorScheme>()
			.unwrap()
			.toggle();
		world.update_local();
		let dark_fg = world
			.entity(entity)
			.get::<VisualStyle>()
			.unwrap()
			.foreground;

		light_fg.is_some().xpect_true();
		(light_fg != dark_fg).xpect_true();
		// the handle also mirrored onto the cascade classes
		world
			.entity(entity)
			.get::<Classes>()
			.unwrap()
			.contains_name(&classes::DARK_SCHEME)
			.xpect_true();
	}
}
