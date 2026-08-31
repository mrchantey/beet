use crate::prelude::*;
use crate::style::material::*;
use crate::style::*;
use beet_core::prelude::*;

/// The active theme: the key colours seeding the Material palettes plus the
/// app-wide [`ColorScheme`] default for non-web targets.
///
/// [`Theme::color`] is the seed every unset key derives from, so a bare
/// `<Theme color=../>` generates a whole palette from one brand colour. A brand
/// that has picked its own key colours overrides them individually, and the
/// neutral key is split per mode so a light scheme can be cream while its dark
/// twin is a green-cast near-black (see [`themes::from_theme`]). The scheme is
/// the session default a non-html target (the terminal) falls back to when a
/// request pins none, eg seeded from a `--color-scheme` CLI argument by the live
/// TUI.
///
/// Declared in markup as `<Theme color="#006c4f" secondary="#f028a8"/>` — the
/// resolver patches the live resource exactly like `<PackageConfig/>` — or
/// inserted in Rust. A change re-runs [`rebuild_theme_tones`], rewriting the
/// `:root` tone declarations so both the web CSS bake and the charcell cascade
/// recolour from the same keys.
///
/// Always present once [`MaterialStylePlugin`] is added (it `init_resource`s the
/// default), so consumers read `Res<Theme>` directly.
#[derive(Debug, Clone, PartialEq, Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct Theme {
	/// The seed colour every unset palette key derives from.
	pub color: Color,
	/// The key colour of the primary palette, the brand's dominant accent.
	pub primary: Option<Color>,
	/// The key colour of the secondary palette, the supporting accent.
	pub secondary: Option<Color>,
	/// The key colour of the tertiary palette, the contrasting accent.
	pub tertiary: Option<Color>,
	/// The key colour seeding light-scheme surfaces and dark-scheme inverse
	/// surfaces.
	pub neutral_light: Option<Color>,
	/// The key colour seeding dark-scheme surfaces and light-scheme inverse
	/// surfaces.
	pub neutral_dark: Option<Color>,
	/// The key colour of the neutral variant palette, the outlines and the
	/// medium-emphasis surfaces.
	pub neutral_variant: Option<Color>,
	/// The key colour of the error palette.
	pub error: Option<Color>,
	/// The app-wide default colour scheme for non-web targets.
	pub scheme: ColorScheme,
}

impl Default for Theme {
	fn default() -> Self {
		Self {
			// the historical brand green the plugin baked, so a host that never
			// touches the theme renders the framework's brand by default. The
			// derived `Background`/`Surface` tones stay near-neutral (a faint green
			// tint), conservative on both the web and the terminal. An app picks its
			// own brand by inserting a seeded `Theme` (or `<Theme color=…/>`).
			color: palettes::basic::GREEN.into(),
			primary: None,
			secondary: None,
			tertiary: None,
			neutral_light: None,
			neutral_dark: None,
			neutral_variant: None,
			error: None,
			scheme: ColorScheme::Dark,
		}
	}
}

/// Installs the Material rule set. The palette keys and scheme are owned by the
/// [`Theme`] resource (insert it before adding this plugin to override the
/// default), the only way to configure them.
#[derive(Default)]
pub struct MaterialStylePlugin;

impl Plugin for MaterialStylePlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<CssPlugin>()
			.register_type::<Theme>()
			.init_resource::<Theme>();
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
		// of truth (`from_theme(&Theme)`):
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

/// Rewrite the `:root` palette tones from the live [`Theme`] keys.
///
/// [`themes::from_theme`] is the only colour-dependent piece of the `:root`
/// default; this writes its ~95 tone declarations into the default rule, keyed
/// by token so it overwrites in place (idempotent — the scheme/opacity/
/// typography keys are untouched). Runs whenever [`Theme`] changes.
pub(crate) fn rebuild_theme_tones(
	theme: Res<Theme>,
	mut rules: ResMut<RuleSet>,
) {
	rules
		.default_rule_mut()
		.push_declarations(Rule::new().with_extend(themes::from_theme(&theme)));
}

pub(crate) fn default_token_map() -> CssTokenMap {
	CssTokenMap::default()
		.with_extend(tones::token_map())
		.with_extend(colors::token_map())
		.with_extend(geometry::token_map())
		.with_extend(motion::token_map())
		.with_extend(typography::token_map())
}

/// The Material component rules: the user-agent [`non_visual_rule`] (so
/// metadata/scripting tags resolve to `display: none`), the component
/// [`classes::all_rules`], and the light/dark scheme rules.
pub(crate) fn default_material_rules() -> Vec<Rule> {
	core::iter::once(non_visual_rule())
		.chain(classes::all_rules())
		.chain([themes::light_scheme(), themes::dark_scheme()])
		.collect()
}

/// The colour-**independent** half of the `:root` default: the light scheme
/// token bindings, opacities, typography, geometry, and motion. These never
/// change with the seed colour, so [`MaterialStylePlugin`] bakes them once and
/// lets [`rebuild_theme_tones`] own the colour-dependent tones.
pub(crate) fn scheme_independent_declarations() -> Rule {
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

	/// The [`Theme`] default is the historical brand green the plugin bakes, so a
	/// host that never touches the theme renders the framework brand by default. An
	/// app with its own brand inserts a seeded `Theme`.
	#[beet_core::test]
	fn theme_default_is_brand_green() {
		Theme::default()
			.color
			.xpect_eq(palettes::basic::GREEN.into());
	}

	/// The colour a [`Theme`] derives for one palette tone, the expected value
	/// every assertion below compares a resolved role against.
	fn tone(theme: &Theme, key: impl Into<Token>) -> Color {
		Rule::new()
			.with_extend(themes::from_theme(theme))
			.get_typed::<Color>(&key.into())
			.unwrap()
	}

	/// A theme seeded by colour alone, the do-nothing shape every host gets.
	fn seeded(color: Color) -> Theme { Theme { color, ..default() } }

	/// Setting [`Theme::color`] and running [`rebuild_theme_tones`] rewrites the
	/// `:root` palette tones to exactly `from_theme(that theme)`, and a different
	/// seed yields different tones.
	#[beet_core::test]
	fn theme_recolors_root_tones() {
		let violet = seeded(Color::srgb(0.5, 0.0, 1.0));
		let mut world = MaterialStylePlugin::world();
		world.insert_resource(violet.clone());
		world.run_system_cached(rebuild_theme_tones).unwrap();

		world.with_state::<Res<RuleSet>, _>(|rules| {
			let root = rules.default_rule();
			// every tone the seed produces is resident in the `:root` default
			themes::from_theme(&violet)
				.iter()
				.all(|(key, _)| root.contains_key(key))
				.xpect_true();
			// a representative tone matches the seed's derived value ...
			root.get_typed::<Color>(&tones::Primary40.into())
				.unwrap()
				.xpect_eq(tone(&violet, tones::Primary40));
			// ... and differs from a different seed
			(tone(&violet, tones::Primary40)
				!= tone(&seeded(Color::srgb(1.0, 0.5, 0.0)), tones::Primary40))
			.xpect_true();
		});
	}

	/// An unset key derives from the seed, so a bare `<Theme color=../>` is
	/// unchanged by the split palette, and setting a key moves only that palette.
	#[beet_core::test]
	fn palette_keys_override_the_seed() {
		let seed = seeded(Color::srgb(0.0, 1.0, 0.75));
		let pink = Color::srgb_u8(0xf0, 0x28, 0xa8);
		let keyed = Theme {
			secondary: Some(pink),
			..seed.clone()
		};
		// the keyed palette moves off the seed-derived one ...
		(tone(&keyed, tones::Secondary40) != tone(&seed, tones::Secondary40))
			.xpect_true();
		// ... every other palette is untouched, still deriving from the seed.
		tone(&keyed, tones::Primary40).xpect_eq(tone(&seed, tones::Primary40));
		tone(&keyed, tones::Error40).xpect_eq(tone(&seed, tones::Error40));
		// a key holds its own hue, so tone 40 of the pink stays raspberry rather
		// than sliding purple: red dominates blue dominates green.
		let raspberry = tone(&keyed, tones::Secondary40).to_srgba();
		(raspberry.red > raspberry.blue && raspberry.blue > raspberry.green)
			.xpect_true();
	}

	/// The neutral key is split per mode: the light ramp is the cream, the dark
	/// ramp is the green-cast ink, and the inverse-surface roles cross over so an
	/// inverse surface renders as the other mode's surface.
	#[beet_core::test]
	fn split_neutral_ramps_per_mode() {
		let theme = Theme {
			neutral_light: Some(Color::srgb_u8(0xf4, 0xee, 0xde)),
			neutral_dark: Some(Color::srgb_u8(0x14, 0x21, 0x1d)),
			..seeded(Color::srgb(0.0, 1.0, 0.75))
		};
		// the light surface tone is warm (the cream leans red over blue) ...
		let cream = tone(&theme, tones::NeutralLight94).to_srgba();
		(cream.red > cream.blue).xpect_true();
		// ... and the dark one is green-cast (green over red, and dark).
		let ink = tone(&theme, tones::NeutralDark8).to_srgba();
		(ink.green > ink.red && ink.green < 0.2).xpect_true();
		// the two ramps genuinely differ at every shared tone
		(tone(&theme, tones::NeutralLight50)
			!= tone(&theme, tones::NeutralDark50))
		.xpect_true();
	}

	/// Each scheme's surface roles resolve through its OWN neutral ramp while the
	/// inverse-surface roles cross over to the other mode's, so a light page is
	/// cream with an ink inverse and a dark page is ink with a cream inverse.
	#[beet_core::test]
	fn schemes_resolve_their_own_neutral_ramp() {
		let theme = Theme {
			neutral_light: Some(Color::srgb_u8(0xf4, 0xee, 0xde)),
			neutral_dark: Some(Color::srgb_u8(0x14, 0x21, 0x1d)),
			..default()
		};
		let mut world = MaterialStylePlugin::world();
		world.insert_resource(theme.clone());
		world.run_system_cached(rebuild_theme_tones).unwrap();
		let light = world
			.spawn((rsx! { <div/> }, Classes::new([classes::LIGHT_SCHEME])))
			.id();
		let dark = world
			.spawn((rsx! { <div/> }, Classes::new([classes::DARK_SCHEME])))
			.id();

		world.with_state::<RuleSetQuery, _>(|query| {
			let memo = &mut default();
			// each scheme's surface is its own ramp's surface tone ...
			query
				.resolve(light, colors::Background, memo)
				.unwrap()
				.xpect_eq(tone(&theme, tones::NeutralLight94));
			query
				.resolve(dark, colors::Background, memo)
				.unwrap()
				.xpect_eq(tone(&theme, tones::NeutralDark8));
			// ... and each inverse surface crosses over to the other ramp.
			query
				.resolve(light, colors::InverseSurface, memo)
				.unwrap()
				.xpect_eq(tone(&theme, tones::NeutralDark20));
			query
				.resolve(dark, colors::InverseSurface, memo)
				.unwrap()
				.xpect_eq(tone(&theme, tones::NeutralLight90));
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

	/// The page/card surface fills are ungated, so the built CSS paints them on
	/// both targets: the charcell cascade (which skips `@media`-gated rules) reads
	/// the same `.page`/`.card-filled` backgrounds as the web. The page sits on the
	/// conservative `Background` role, the card on a distinct surface tone.
	#[beet_core::test]
	fn surface_fills_paint_on_both_targets() {
		let builder = CssBuilder::default()
			.with_format_variables(FormatVariables::short());
		MaterialStylePlugin::world()
			.with_state::<StyleQuery, _>(|query| query.build_css(&builder))
			.xunwrap()
			.xpect_contains(".page")
			.xpect_contains("var(--material-colors-background)")
			.xpect_contains(".card-filled")
			.xpect_contains("var(--material-colors-surface-container-highest)");
	}

	/// A deep `.card-filled` under a `.dark-scheme` ancestor resolves the card
	/// surface tone (`SurfaceContainerHighest`, which the web fill points at) to
	/// the dark value, not the light `:root` fallback — the "white card on a dark
	/// page" bug. Asserted on the token rather than the now-web-only background
	/// property, so it holds on both targets.
	#[beet_core::test]
	fn nested_card_inherits_dark_scheme() {
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
		// a bare element with no scheme ancestor falls back to the light `:root`.
		let bare = world.spawn(rsx! { <div/> }).id();
		world.with_state::<RuleSetQuery, _>(|query| {
			let memo = &mut default();
			let card_tone = query
				.resolve(card, colors::SurfaceContainerHighest, memo)
				.unwrap();
			// the nested card inherits the dark ancestor's tone ...
			card_tone.xpect_eq(
				query
					.resolve(body, colors::SurfaceContainerHighest, memo)
					.unwrap(),
			);
			// ... not the light `:root` fallback a bare element resolves.
			(card_tone
				!= query
					.resolve(bare, colors::SurfaceContainerHighest, memo)
					.unwrap())
			.xpect_true();
		});
	}

	/// Content transcluded into a `.dark-scheme` layout by [`Portal`] (no
	/// `ChildOf` edge) still inherits the layout's scheme through the holder, so a
	/// card in referenced content resolves the dark surface tone, not the light
	/// `:root` fallback. This is the document-layout transclusion path that
	/// produced the "white card". Asserted on the surface token (the web fill
	/// points at it) rather than the now-web-only background property.
	#[beet_core::test]
	fn render_ref_content_inherits_dark_scheme() {
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
			let card_tone = query
				.resolve(card, colors::SurfaceContainerHighest, memo)
				.unwrap();
			let dark_highest = query
				.resolve(body, colors::SurfaceContainerHighest, memo)
				.unwrap();
			card_tone.xpect_eq(dark_highest);
		});
	}

	/// A descendant with no scheme class of its own inherits the nearest
	/// ancestor's scheme through the cascade, overriding the light `:root`
	/// default.
	#[beet_core::test]
	fn descendant_inherits_scheme_class() {
		let mut world = MaterialStylePlugin::world();
		let parent = world
			.spawn((rsx! { <div/> }, Classes::new([classes::DARK_SCHEME])))
			.id();
		let child = world.spawn((rsx! { <span/> }, ChildOf(parent))).id();
		// a sibling with no scheme falls back to the light `:root` default
		let bare = world.spawn(rsx! { <span/> }).id();

		world.with_state::<RuleSetQuery, _>(|query| {
			let memo = &mut default();
			let child_surface =
				query.resolve(child, colors::Surface, memo).unwrap();
			// inherits the parent's dark scheme ...
			child_surface.xpect_eq(
				query.resolve(parent, colors::Surface, memo).unwrap(),
			);
			// ... which differs from the do-nothing light fallback
			(child_surface
				!= query.resolve(bare, colors::Surface, memo).unwrap())
			.xpect_true();
		});
	}

	/// End-to-end: a `.page` root carrying a scheme class resolves the scheme's
	/// `Background` base and foreground on the charcell cascade too (the fill is
	/// ungated, so the terminal paints it), and the light and dark schemes resolve
	/// to different tones.
	#[beet_core::test]
	fn scheme_class_themes_page() {
		// `RealtimeParsePlugin` wires `PostParseTree` into the main loop so
		// `update_local` resolves styles (the on-demand render paths run it directly)
		let mut world = (
			MaterialStylePlugin::default(),
			StylePlugin,
			RealtimeParsePlugin,
		)
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
		let light_style =
			world.entity(light).get::<VisualStyle>().unwrap().clone();
		let dark_style =
			world.entity(dark).get::<VisualStyle>().unwrap().clone();

		// the scheme themes the page foreground, differing between schemes ...
		light_style.foreground.is_some().xpect_true();
		(light_style.foreground != dark_style.foreground).xpect_true();
		// ... and the ungated `Background` fill paints on charcell too, its tone
		// differing between schemes.
		light_style.background.is_some().xpect_true();
		(light_style.background != dark_style.background).xpect_true();
	}
}
