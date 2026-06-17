use crate::prelude::*;
use crate::style::material::*;
use crate::style::*;
use beet_core::prelude::*;

/// The brand colour seeding the Material palette, the single source of every
/// accent (every tone derives from it through [`themes::from_color`]).
///
/// Declared in markup as `<Theme color=Srgba{..}/>` — the resolver patches the
/// live resource exactly like `<PackageConfig/>` — or inserted in Rust. A change
/// re-runs [`rebuild_theme_tones`], rewriting the `:root` tone declarations so
/// both the web CSS bake and the charcell cascade recolour from the same seed.
#[derive(Debug, Clone, PartialEq, Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct Theme {
	/// The seed colour the palette tones derive from.
	pub color: Color,
}

impl Default for Theme {
	fn default() -> Self {
		Self {
			color: palettes::basic::GREEN.into(),
		}
	}
}

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
			color: Theme::default().color,
		}
	}
}

impl Plugin for MaterialStylePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<CssPlugin>()
			.register_type::<Theme>()
			.insert_resource(Theme { color: self.color });
		// Extend the existing rule set rather than replacing it, so the prose
		// `default_element_rules` seeded by `StylePlugin` (em → italic, h1 →
		// bold, code/a → inline, …) survive alongside the Material rules. The
		// colour-DEPENDENT `:root` tones are written by `rebuild_theme_tones`
		// (run below), keyed off the live `Theme`; only the colour-INDEPENDENT
		// half (scheme/opacity/typography/geometry/motion) merges in here.
		let mut rules = app.world_mut().get_resource_or_init::<RuleSet>();
		rules
			.default_rule_mut()
			.push_declarations(scheme_independent_declarations());
		rules.extend_rules(default_material_rules());
		app.world_mut()
			.get_resource_or_init::<CssTokenMap>()
			.extend(default_token_map());

		// Derive the `:root` tones from `Theme` and rewrite them on every change
		// (insert or a late `<Theme>` patch). Two trigger schedules, one source
		// of truth (`from_color(Theme.color)`):
		// - `PostParseTree` before the cascade reads the rule set — recolours the
		//   charcell render (every terminal render runs this schedule on demand).
		// - `PreUpdate` — recolours a pure-web build, whose HTML/CSS bake does not
		//   run `PostParseTree`, before the first request reads the rule set.
		app.add_systems(
			PostParseTree,
			rebuild_theme_tones
				.before(ResolveStylesSet)
				.run_if(resource_changed::<Theme>),
		)
		.add_systems(
			PreUpdate,
			rebuild_theme_tones.run_if(resource_changed::<Theme>),
		);
		// seed the tones now from the same system, so the rule set carries them
		// immediately — before any schedule runs (eg a `with_state` style query,
		// or a web bake reading the rule set on the very first request).
		app.world_mut()
			.run_system_cached(rebuild_theme_tones)
			.unwrap();
	}
}

/// Rewrite the `:root` palette tones from the live [`Theme`] seed.
///
/// [`themes::from_color`] is the only colour-dependent piece of the `:root`
/// default; this writes its ~85 tone declarations into the default rule, keyed
/// by token so it overwrites in place (idempotent — the scheme/opacity/
/// typography keys are untouched). Runs whenever [`Theme`] changes.
pub fn rebuild_theme_tones(theme: Res<Theme>, mut rules: ResMut<RuleSet>) {
	rules
		.default_rule_mut()
		.push_declarations(Rule::new().with_extend(themes::from_color(theme.color)));
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
/// it, mirroring `color_scheme.js`. The colour-dependent tones come from the
/// seed via [`themes::from_color`]; the rest from
/// [`scheme_independent_declarations`].
pub fn default_declarations(color: impl Into<Color>) -> Rule {
	scheme_independent_declarations().with_extend(themes::from_color(color))
}

/// The colour-**independent** half of the `:root` default: the light scheme
/// token bindings, opacities, typography, geometry, and motion. These never
/// change with the seed colour, so [`MaterialStylePlugin`] bakes them once and
/// lets [`rebuild_theme_tones`] own the colour-dependent tones.
pub fn scheme_independent_declarations() -> Rule {
	Rule::new()
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

	/// The [`Theme`] default is the historical brand green the plugin baked, so a
	/// host that never touches the theme (and rsx_site, which sets it to its own
	/// green) renders byte-identically across this refactor.
	#[beet_core::test]
	fn theme_default_is_brand_green() {
		Theme::default().color.xpect_eq(palettes::basic::GREEN.into());
	}

	/// Setting [`Theme::color`] and running [`rebuild_theme_tones`] rewrites the
	/// `:root` palette tones to exactly `from_color(that color)`, and a different
	/// seed yields different tones.
	#[beet_core::test]
	fn theme_recolors_root_tones() {
		let violet = Color::srgb(0.5, 0.0, 1.0);
		let mut world = MaterialStylePlugin::world();
		world.insert_resource(Theme { color: violet });
		world.run_system_cached(rebuild_theme_tones).unwrap();

		world.with_state::<Res<RuleSet>, _>(|rules| {
			let root = rules.default_rule();
			// every tone the seed produces is resident in the `:root` default
			themes::from_color(violet)
				.iter()
				.all(|(key, _)| root.contains_key(key))
				.xpect_true();
			// a representative tone matches the seed's derived value ...
			let from_seed = |color| {
				Rule::new()
					.with_extend(themes::from_color(color))
					.get_typed::<Color>(&tones::Primary40.into())
					.unwrap()
			};
			root.get_typed::<Color>(&tones::Primary40.into())
				.unwrap()
				.xpect_eq(from_seed(violet));
			// ... and differs from a different seed
			(from_seed(violet) != from_seed(Color::srgb(1.0, 0.5, 0.0)))
				.xpect_true();
		});
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
