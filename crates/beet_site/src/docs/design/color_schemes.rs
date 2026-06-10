use beet::prelude::*;

/// A single colour-role swatch on the color-schemes showcase.
const COLOR_BOX: ClassName = ClassName::new_static("color-box");
/// A wrapping flex row of [`COLOR_BOX`] swatches.
const COLOR_GROUP: ClassName = ClassName::new_static("color-group");

/// Showcases every Material color role as a swatch, in both the light and dark
/// schemes. Each swatch fills its background with the color-role token and its
/// text with the matching "on" token.
///
/// The swatch colours are token *references* bound by [`color_scheme_rules`],
/// not a web-only `<style>`, so the palette resolves on the terminal too and
/// each `.light-scheme`/`.dark-scheme` wrapper renders its own scheme on both
/// targets.
pub fn get() -> impl Bundle {
	rsx! {
		<article>
			<h1>"Color Schemes"</h1>
			<h2>"Light"</h2>
			<div {Classes::new([classes::LIGHT_SCHEME])}>{scheme()}</div>
			<h2>"Dark"</h2>
			<div {Classes::new([classes::DARK_SCHEME])}>{scheme()}</div>
		</article>
	}
}

/// One full set of color-role swatches, grouped by Material role family.
fn scheme() -> impl Bundle {
	let groups: Vec<_> =
		color_swatch_groups().into_iter().map(swatch_group).collect();
	rsx! { <div>{groups}</div> }
}

/// A titled, wrapping row of swatches for one colour-role family.
fn swatch_group(group: SwatchGroup) -> impl Bundle {
	let title = group.title.to_string();
	let boxes: Vec<_> = group
		.swatches
		.into_iter()
		.map(|swatch| {
			let label = swatch.label.to_string();
			rsx! {
				<div {Classes::new([COLOR_BOX, ClassName::string(swatch.role)])}>
					{label}
				</div>
			}
		})
		.collect();
	rsx! {
		<div>
			<h3>{title}</h3>
			<div {Classes::new([COLOR_GROUP])}>{boxes}</div>
		</div>
	}
}

/// Rules for the colour-schemes showcase: the swatch/box layout plus one
/// token-bound rule per colour role. Token-to-token bindings can't be expressed
/// through `Classes`/`inline_class!` in scene-mode `rsx!`, so the showcase
/// registers these once (in [`server_plugin`](crate::server_plugin)) rather than
/// emitting a web-only `<style>` block the terminal would skip.
pub fn color_scheme_rules() -> Vec<Rule> {
	use style::Direction;
	use style::Display;
	use style::FlexWrap;
	use style::Length;
	use style::Spacing;
	use style::common_props;

	// the row of swatches wraps, with a hair of gap that rounds to zero cells on
	// the terminal and shows as a thin seam on the web.
	let group = Rule::new()
		.with_selector(Selector::class(COLOR_GROUP))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexDirectionProp, Direction::Horizontal)
		.with_value(common_props::FlexWrapProp, FlexWrap::Wrap)
		.with_value(common_props::ColumnGapProp, Length::Px(2.))
		.with_value(common_props::RowGapProp, Length::Px(2.))
		.with_value(common_props::MarginProp, Spacing {
			bottom: Length::Px(2.),
			..Spacing::DEFAULT
		});
	// each swatch is a fixed-width block holding its label over the role fill. It
	// is a block (not a flex box) so the bare text label is a normal inline-flow
	// child the charcell engine paints; a flex container would leave a raw text
	// child unsized and collapse the swatch to nothing.
	let box_rule = Rule::new()
		.with_selector(Selector::class(COLOR_BOX))
		.with_value(common_props::Width, Length::Rem(10.))
		.with_value(common_props::Height, Length::Rem(3.))
		.with_value(common_props::Padding, Spacing::all(Length::Rem(0.3)));

	let mut rules = vec![group, box_rule];
	for group in color_swatch_groups() {
		for swatch in group.swatches {
			rules.push(role_rule(swatch.role, swatch.bg, swatch.fg));
		}
	}
	rules
}

/// A `.color-box.<role>` rule binding the swatch's background and text to its
/// colour-role tokens, scoped to swatches so a generic role word (eg `error`)
/// can't leak onto unrelated elements.
fn role_rule(role: &str, bg: Token, fg: Token) -> Rule {
	Rule::new()
		.with_selector(Selector::AllOf(vec![
			Selector::class(COLOR_BOX),
			Selector::class(ClassName::string(role)),
		]))
		.with_token(style::common_props::BackgroundColor, bg)
		.unwrap()
		.with_token(style::common_props::ForegroundColor, fg)
		.unwrap()
}

/// One labelled colour-role swatch: its display label, role class, and the
/// background/foreground design tokens it binds.
///
/// `bg`/`fg` are token *references*, so the cascade resolves them against the
/// nearest scheme on both targets — the same swatch reads light under
/// `.light-scheme` and dark under `.dark-scheme`, on the web and the terminal.
struct Swatch {
	label: &'static str,
	role: &'static str,
	bg: Token,
	fg: Token,
}

/// A titled group of related colour roles (eg the primary family).
struct SwatchGroup {
	title: &'static str,
	swatches: Vec<Swatch>,
}

/// Build one swatch from its label, role class, and background/"on" tokens.
fn swatch(
	label: &'static str,
	role: &'static str,
	bg: impl Into<Token>,
	fg: impl Into<Token>,
) -> Swatch {
	Swatch { label, role, bg: bg.into(), fg: fg.into() }
}

/// Every Material colour role grouped by family, the single source of truth for
/// both the showcase DOM and its rules.
fn color_swatch_groups() -> Vec<SwatchGroup> {
	use material::colors::*;
	vec![
		SwatchGroup {
			title: "Primary",
			swatches: vec![
				swatch("Primary", "primary", Primary, OnPrimary),
				swatch("On Primary", "on-primary", OnPrimary, Primary),
				swatch("Primary Container", "primary-container", PrimaryContainer, OnPrimaryContainer),
				swatch("On Primary Container", "on-primary-container", OnPrimaryContainer, PrimaryContainer),
				swatch("Inverse Primary", "inverse-primary", InversePrimary, InverseSurface),
			],
		},
		SwatchGroup {
			title: "Secondary",
			swatches: vec![
				swatch("Secondary", "secondary", Secondary, OnSecondary),
				swatch("On Secondary", "on-secondary", OnSecondary, Secondary),
				swatch("Secondary Container", "secondary-container", SecondaryContainer, OnSecondaryContainer),
				swatch("On Secondary Container", "on-secondary-container", OnSecondaryContainer, SecondaryContainer),
			],
		},
		SwatchGroup {
			title: "Tertiary",
			swatches: vec![
				swatch("Tertiary", "tertiary", Tertiary, OnTertiary),
				swatch("On Tertiary", "on-tertiary", OnTertiary, Tertiary),
				swatch("Tertiary Container", "tertiary-container", TertiaryContainer, OnTertiaryContainer),
				swatch("On Tertiary Container", "on-tertiary-container", OnTertiaryContainer, TertiaryContainer),
			],
		},
		SwatchGroup {
			title: "Error",
			swatches: vec![
				swatch("Error", "error", Error, OnError),
				swatch("On Error", "on-error", OnError, Error),
				swatch("Error Container", "error-container", ErrorContainer, OnErrorContainer),
				swatch("On Error Container", "on-error-container", OnErrorContainer, ErrorContainer),
			],
		},
		SwatchGroup {
			title: "Surface",
			swatches: vec![
				swatch("Surface Dim", "surface-dim", SurfaceDim, OnSurface),
				swatch("Surface", "surface", Surface, OnSurface),
				swatch("Surface Bright", "surface-bright", SurfaceBright, OnSurface),
				swatch("On Surface", "on-surface", OnSurface, Surface),
				swatch("Surface Variant", "surface-variant", SurfaceVariant, OnSurfaceVariant),
				swatch("On Surface Variant", "on-surface-variant", OnSurfaceVariant, SurfaceVariant),
				swatch("Inverse Surface", "inverse-surface", InverseSurface, InverseOnSurface),
				swatch("Inverse On Surface", "inverse-on-surface", InverseOnSurface, InverseSurface),
				swatch("Surface Container Lowest", "surface-container-lowest", SurfaceContainerLowest, OnSurface),
				swatch("Surface Container Low", "surface-container-low", SurfaceContainerLow, OnSurface),
				swatch("Surface Container", "surface-container", SurfaceContainer, OnSurface),
				swatch("Surface Container High", "surface-container-high", SurfaceContainerHigh, OnSurface),
				swatch("Surface Container Highest", "surface-container-highest", SurfaceContainerHighest, OnSurface),
			],
		},
		SwatchGroup {
			title: "Misc",
			swatches: vec![
				swatch("Outline", "outline", Outline, Surface),
				swatch("Outline Variant", "outline-variant", OutlineVariant, OnSurface),
				swatch("Scrim", "scrim", Scrim, Surface),
				swatch("Shadow", "shadow", Shadow, Surface),
			],
		},
	]
}
