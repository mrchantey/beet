use crate::prelude::*;
use crate::style::material::*;
use crate::style::*;
use beet_core::prelude::*;

pub struct MaterialStylePlugin {
	color: Color,
}

impl MaterialStylePlugin {
	pub fn new(color: impl Into<Color>) -> Self {
		Self {
			color: color.into(),
		}
	}
}

impl Default for MaterialStylePlugin {
	fn default() -> Self {
		Self {
			color: palettes::basic::GREEN.into(),
		}
	}
}

impl Plugin for MaterialStylePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<CssPlugin>()
			.insert_resource(default_rule_set(self.color.clone()));
		app.world_mut()
			.get_resource_or_init::<CssTokenMap>()
			.extend(default_token_map());
	}
}

pub fn default_token_map() -> CssTokenMap {
	CssTokenMap::default()
		.with_extend(tones::token_map())
		.with_extend(colors::token_map())
		.with_extend(geometry::token_map())
		.with_extend(motion::token_map())
		.with_extend(typography::token_map())
}

/// All default material declarations and component rules.
pub fn default_rule_set(color: impl Into<Color>) -> RuleSet {
	RuleSet::new(default_declarations(color))
		.with_rules(rules::all_rules())
		.with_rule(themes::light_scheme())
		.with_rule(themes::dark_scheme())
}

/// Returns a [`Rule`] with all material design default values.
///
/// This is the `:root` rule — the lowest-priority fallback in the cascade. It
/// bakes in the **light** scheme so a document with no scheme class still gets
/// colors; a `.dark-scheme` (or `.light-scheme`) class on an ancestor overrides
/// it, mirroring `color_scheme.js`.
pub fn default_declarations(color: impl Into<Color>) -> Rule {
	Rule::new()
		.with_extend(themes::from_color(color))
		.with_extend(themes::light_scheme())
		.with_extend(themes::default_opacities())
		.with_extend(typography::default_typography())
		.with_extend(geometry::default_shapes())
		.with_extend(geometry::default_elevations())
		.with_extend(motion::default_durations())
		.with_extend(motion::default_motions())
}


#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test]
	fn material_rule_set() {
		MaterialStylePlugin::world()
			.with_state::<Res<RuleSet>, _>(|rules| {
				// OnPrimary lives in the scheme rules (and the light `:root`
				// default), so some rule resolves it
				rules
					.iter()
					.any(|rule| rule.get(&colors::OnPrimary.into()).is_ok())
			})
			.xpect_true();
	}
	#[beet_core::test]
	fn material_css() {
		MaterialStylePlugin::world()
			.with_state::<StyleQuery, _>(|query| {
				query.build_css(&default())
			})
			.xunwrap()
			.xpect_contains(
				"--io-crates-beet-ui-style-material-motion-short2: 100ms;",
			)
			.xpect_contains("--io-crates-beet-ui-style-material-typography-headline-large-weight: var(--io-crates-beet-ui-style-material-typography-weight-regular);");
	}

	/// A descendant with no scheme class of its own inherits the nearest
	/// ancestor's scheme through the cascade, overriding the light `:root`
	/// default.
	#[beet_core::test]
	fn descendant_inherits_scheme_class() {
		let mut world = MaterialStylePlugin::world();
		let parent = world
			.spawn((rsx_direct! { <div/> }, Classes::new([classes::DARK_SCHEME])))
			.id();
		let child = world.spawn((rsx_direct! { <span/> }, ChildOf(parent))).id();
		// a sibling with no scheme falls back to the light `:root` default
		let bare = world.spawn(rsx_direct! { <span/> }).id();

		world.with_state::<RuleSetQuery, _>(|query| {
			let child_surface = query.resolve(child, colors::Surface).unwrap();
			// inherits the parent's dark scheme ...
			child_surface
				.xpect_eq(query.resolve(parent, colors::Surface).unwrap());
			// ... which differs from the do-nothing light fallback
			(child_surface != query.resolve(bare, colors::Surface).unwrap())
				.xpect_true();
		});
	}

	/// End-to-end: a `.page` root carrying a scheme class resolves a background,
	/// and the light and dark schemes resolve to different colors.
	#[beet_core::test]
	fn scheme_class_themes_page() {
		let mut world =
			(MaterialStylePlugin::default(), StylePlugin).into_world();
		let light = world
			.spawn((
				rsx_direct! { <div/> },
				Classes::new([classes::PAGE, classes::LIGHT_SCHEME]),
			))
			.id();
		let dark = world
			.spawn((
				rsx_direct! { <div/> },
				Classes::new([classes::PAGE, classes::DARK_SCHEME]),
			))
			.id();

		world.update_local();
		let light_bg =
			world.entity(light).get::<VisualStyle>().unwrap().background;
		let dark_bg =
			world.entity(dark).get::<VisualStyle>().unwrap().background;

		light_bg.is_some().xpect_true();
		(light_bg != dark_bg).xpect_true();
	}
}
