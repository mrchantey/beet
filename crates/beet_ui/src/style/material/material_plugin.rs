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
		app.init_plugin::<CssPlugin>();
		// Extend the existing rule set rather than replacing it, so the prose
		// `default_element_rules` seeded by `StylePlugin` (em → italic, h1 →
		// bold, code/a → inline, …) survive alongside the Material rules. The
		// `:root` defaults merge into the shared default rule.
		let mut rules = app.world_mut().get_resource_or_init::<RuleSet>();
		rules
			.default_rule_mut()
			.push_declarations(default_declarations(self.color.clone()));
		rules.extend_rules(default_material_rules());
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

/// All default material declarations and component rules, as a standalone rule
/// set (no prose [`default_element_rules`]). [`MaterialStylePlugin`] instead
/// extends the shared rule set so it composes with `StylePlugin`'s prose rules.
pub fn default_rule_set(color: impl Into<Color>) -> RuleSet {
	RuleSet::new(default_declarations(color))
		.with_rules(default_material_rules())
}

/// The Material component rules: the user-agent [`non_visual_rule`] (so
/// metadata/scripting tags resolve to `display: none`), the component
/// [`classes::all_rules`], and the light/dark scheme rules.
pub fn default_material_rules() -> Vec<Rule> {
	core::iter::once(non_visual_rule())
		.chain(classes::all_rules())
		.chain([themes::light_scheme(), themes::dark_scheme()])
		.collect()
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

	/// A deep `.card-filled` under a `.dark-scheme` ancestor resolves its
	/// `BackgroundColor` (which points at `SurfaceContainerHighest`) to the dark
	/// tone, not the light `:root` fallback — the "white card on a dark page" bug.
	#[beet_core::test]
	fn nested_card_inherits_dark_scheme() {
		use crate::style::common_props::BackgroundColor;
		let mut world = MaterialStylePlugin::world();
		let body = world
			.spawn((rsx! { <div/> }, Classes::new([classes::DARK_SCHEME])))
			.id();
		let mid = world.spawn((rsx! { <main/> }, ChildOf(body))).id();
		let card = world
			.spawn((
				rsx! { <div/> },
				Classes::new([classes::CARD_FILLED]),
				ChildOf(mid),
			))
			.id();
		world.with_state::<RuleSetQuery, _>(|query| {
			let memo = &mut default();
			let card_bg = query.resolve(card, BackgroundColor, memo).unwrap();
			let dark_highest = query
				.resolve(body, colors::SurfaceContainerHighest, memo)
				.unwrap();
			card_bg.xpect_eq(dark_highest);
		});
	}

	/// Content transcluded into a `.dark-scheme` layout by [`Portal`] (no
	/// `ChildOf` edge) still inherits the layout's scheme through the holder, so a
	/// card in referenced content is dark, not the light `:root` fallback. This is
	/// the document-layout transclusion path that produced the "white card".
	#[beet_core::test]
	fn render_ref_content_inherits_dark_scheme() {
		use crate::style::common_props::BackgroundColor;
		let mut world = MaterialStylePlugin::world();
		// content is its own root (no ChildOf to the layout), holding a card
		let content = world.spawn(rsx! { <main/> }).id();
		let card = world
			.spawn((
				rsx! { <div/> },
				Classes::new([classes::CARD_FILLED]),
				ChildOf(content),
			))
			.id();
		// layout body carries the scheme; a holder transcludes the content by ref
		let body = world
			.spawn((rsx! { <div/> }, Classes::new([classes::DARK_SCHEME])))
			.id();
		world.spawn((Portal::new(content), ChildOf(body)));
		world.with_state::<RuleSetQuery, _>(|query| {
			let memo = &mut default();
			let card_bg = query.resolve(card, BackgroundColor, memo).unwrap();
			let dark_highest = query
				.resolve(body, colors::SurfaceContainerHighest, memo)
				.unwrap();
			card_bg.xpect_eq(dark_highest);
		});
	}

	/// A descendant with no scheme class of its own inherits the nearest
	/// ancestor's scheme through the cascade, overriding the light `:root`
	/// default.
	#[beet_core::test]
	fn descendant_inherits_scheme_class() {
		let mut world = MaterialStylePlugin::world();
		let parent = world
			.spawn((
				rsx! { <div/> },
				Classes::new([classes::DARK_SCHEME]),
			))
			.id();
		let child =
			world.spawn((rsx! { <span/> }, ChildOf(parent))).id();
		// a sibling with no scheme falls back to the light `:root` default
		let bare = world.spawn(rsx! { <span/> }).id();

		world.with_state::<RuleSetQuery, _>(|query| {
			let memo = &mut default();
			let child_surface =
				query.resolve(child, colors::Surface, memo).unwrap();
			// inherits the parent's dark scheme ...
			child_surface
				.xpect_eq(query.resolve(parent, colors::Surface, memo).unwrap());
			// ... which differs from the do-nothing light fallback
			(child_surface
				!= query.resolve(bare, colors::Surface, memo).unwrap())
			.xpect_true();
		});
	}

	/// End-to-end: a `.page` root carrying a scheme class resolves a background,
	/// and the light and dark schemes resolve to different colors.
	#[beet_core::test]
	fn scheme_class_themes_page() {
		// `RealtimeParsePlugin` wires `PostParseTree` into the main loop so
		// `update_local` resolves styles (the on-demand render paths run it directly)
		let mut world =
			(MaterialStylePlugin::default(), StylePlugin, RealtimeParsePlugin)
				.into_world();
		let light = world
			.spawn((
				rsx! { <div/> },
				Classes::new([classes::PAGE, classes::LIGHT_SCHEME]),
			))
			.id();
		let dark = world
			.spawn((
				rsx! { <div/> },
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
