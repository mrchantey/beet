//! Site-local style: the rules layered onto the library Material set.
use beet::prelude::*;

/// The site's own class names, layered on the library
/// [`classes`](beet::prelude::classes).
///
/// Note: inside `rsx!` a bare `classes::` always resolves to the *library*
/// module (the macro injects `beet_ui::prelude::*`), so a site-local class is
/// referenced by its full path, eg `crate::style::classes::DESIGN_ROW`.
pub mod classes {
	use beet::prelude::ClassName;

	/// Site-local class for the design showcase rows that lay widget variants
	/// out side by side.
	pub const DESIGN_ROW: ClassName = ClassName::new_static("design-row");
	/// A single colour-role swatch on the color-schemes showcase.
	pub const COLOR_BOX: ClassName = ClassName::new_static("color-box");
	/// A wrapping flex row of [`COLOR_BOX`] swatches.
	pub const COLOR_GROUP: ClassName = ClassName::new_static("color-group");
}

/// A horizontal flex row with a gap, for the design showcase pages that lay out
/// widget variants side by side, styling [`classes::DESIGN_ROW`].
///
/// Expressed as design tokens rather than a raw `<style>` so it spaces items in
/// both the web and terminal targets, mirroring the library `app-bar-nav` rule.
pub fn design_row_rule() -> Rule {
	use style::AlignItems;
	use style::Display;
	use style::FlexWrap;
	use style::Length;
	use style::common_props;
	// the `Length` row/column gap props serialize to valid CSS *and* drive the
	// charcell flex layout, so one value spaces items on both targets.
	Rule::new()
		.with_selector(Selector::class(classes::DESIGN_ROW))
		.with_value(common_props::DisplayProp, Display::Flex)
		.with_value(common_props::FlexWrapProp, FlexWrap::Wrap)
		.with_value(common_props::AlignItemsProp, AlignItems::Center)
		.with_value(common_props::ColumnGapProp, Length::Rem(1.0))
		.with_value(common_props::RowGapProp, Length::Rem(1.0))
}

// ── Colour-scheme showcase ──────────────────────────────────────────────────

/// One labelled colour-role swatch: its display label, role class, and the
/// background/foreground design tokens it binds.
///
/// `bg`/`fg` are token *references*, so the cascade resolves them against the
/// nearest scheme on both targets — the same swatch reads light under
/// `.light-scheme` and dark under `.dark-scheme`, on the web and the terminal.
pub struct Swatch {
	pub label: &'static str,
	pub role: &'static str,
	pub bg: Token,
	pub fg: Token,
}

/// A titled group of related colour roles (eg the primary family).
pub struct SwatchGroup {
	pub title: &'static str,
	pub swatches: Vec<Swatch>,
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
/// both the showcase DOM (`docs/design/color_schemes`) and its rules.
pub fn color_swatch_groups() -> Vec<SwatchGroup> {
	use beet::ui::style::material::colors::*;
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
		.with_selector(Selector::class(classes::COLOR_GROUP))
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
		.with_selector(Selector::class(classes::COLOR_BOX))
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
			Selector::class(classes::COLOR_BOX),
			Selector::class(ClassName::string(role)),
		]))
		.with_token(common_props::BackgroundColor, bg)
		.unwrap()
		.with_token(common_props::ForegroundColor, fg)
		.unwrap()
}
