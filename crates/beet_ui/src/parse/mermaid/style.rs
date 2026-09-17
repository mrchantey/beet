#![cfg_attr(rustfmt, rustfmt_skip)]
//! The diagram classes and the rules that style them, colocated with the
//! passes that emit them.
use crate::prelude::*;
use crate::style::common_props;
// explicit imports shadow the bevy_ui twins `beet_core::prelude` re-exports
// under `bevy_default`
use crate::style::Display;
use crate::style::Overflow;
use beet_core::prelude::*;
use crate::style::material::*;
use crate::style::*;
// explicit: bevy's ui prelude names both under `bevy_default`
use crate::style::Display;
use crate::style::Overflow;

/// The figure a diagram renders into, on the code-block surface.
pub const DIAGRAM: ClassName = ClassName::new_static("diagram");
/// The `<pre>` holding a diagram's box-drawing text.
pub const DIAGRAM_TEXT: ClassName = ClassName::new_static("diagram-text");

/// The surface a diagram paints on: the `pre` fill, so a diagram and a code
/// block on one page share a surface. The figure rule and the svg theme's
/// background both read it.
pub(crate) fn diagram_surface() -> Token { colors::SurfaceContainerHighest.into() }

/// The rules styling [`DIAGRAM`], its svg and [`DIAGRAM_TEXT`] on both sinks,
/// and the paint roles on each scheme class.
pub(crate) fn diagram_rules() -> Vec<Rule> {
	vec![
		diagram_figure(),
		diagram_svg(),
		diagram_text(),
		scheme_paint(classes::LIGHT_SCHEME),
		scheme_paint(classes::DARK_SCHEME),
	]
}

/// The material defaults of every [`DiagramPaint`] role, declared on `:root`
/// so a page root's `bx:style="diagram-node-fill=@token:TertiaryContainer"`
/// still wins for the diagrams below it (an inherited value beats nothing, a
/// `.diagram` declaration would beat the page), and again on each scheme
/// class ([`diagram_rules`]): a redirect resolves where it is declared, as a
/// css custom property does, so the `:root` declaration alone pins the light
/// tones under a `.dark-scheme` body. The svg theme reads these by `var()` on
/// the web and by resolved hex on a terminal.
pub(crate) fn diagram_paint_defaults() -> Rule {
	Rule::new()
		.with_token(common_props::DiagramSurfaceProp, diagram_surface()).unwrap()
		.with_token(common_props::DiagramNodeFillProp, colors::PrimaryContainer).unwrap()
		.with_token(common_props::DiagramNodeTextProp, colors::OnPrimaryContainer).unwrap()
		.with_token(common_props::DiagramOutlineProp, colors::Outline).unwrap()
		.with_token(common_props::DiagramLineProp, colors::OnSurfaceVariant).unwrap()
		.with_token(common_props::DiagramTextProp, colors::OnSurface).unwrap()
		.with_token(common_props::DiagramSecondaryFillProp, colors::SecondaryContainer).unwrap()
		.with_token(common_props::DiagramTertiaryFillProp, colors::TertiaryContainer).unwrap()
		.with_token(common_props::DiagramTertiaryTextProp, colors::OnTertiaryContainer).unwrap()
		.with_token(common_props::DiagramClusterFillProp, colors::SurfaceContainer).unwrap()
		.with_token(common_props::DiagramClusterOutlineProp, colors::OutlineVariant).unwrap()
		.with_token(common_props::DiagramFontProp, typography::TypefacePlain).unwrap()
		.with_value(common_props::DiagramRampProp, material_ramp())
}

/// The tonal ramp behind pie slices and git branches, `(fill, on)` pairs across
/// the three accents: containers, then the fixed tones, then the accents.
fn material_ramp() -> DiagramRamp {
	[
		(colors::PrimaryContainer.into(),   colors::OnPrimaryContainer.into()),
		(colors::SecondaryContainer.into(), colors::OnSecondaryContainer.into()),
		(colors::TertiaryContainer.into(),  colors::OnTertiaryContainer.into()),
		(colors::PrimaryFixedDim.into(),    colors::OnPrimaryFixed.into()),
		(colors::SecondaryFixedDim.into(),  colors::OnSecondaryFixed.into()),
		(colors::TertiaryFixedDim.into(),   colors::OnTertiaryFixed.into()),
		(colors::PrimaryFixed.into(),       colors::OnPrimaryFixedVariant.into()),
		(colors::SecondaryFixed.into(),     colors::OnSecondaryFixedVariant.into()),
		(colors::TertiaryFixed.into(),      colors::OnTertiaryFixedVariant.into()),
		(colors::Primary.into(),            colors::OnPrimary.into()),
		(colors::Secondary.into(),          colors::OnSecondary.into()),
		(colors::Tertiary.into(),           colors::OnTertiary.into()),
	]
	.into_iter()
	.map(|(background, foreground)| ColorRole { background, foreground })
	.collect::<Vec<_>>()
	.xmap(DiagramRamp)
}

/// The paint defaults on a scheme class, where `PrimaryContainer` and the
/// rest resolve to that scheme's tones.
fn scheme_paint(scheme: ClassName) -> Rule {
	diagram_paint_defaults().with_selector(Selector::class(scheme))
}

/// `.diagram` - the `pre` surface (fill, padding, corner), so a diagram and a
/// code block on one page share a look, a block gap below, and a horizontal
/// scroll on the web for art wider than the column.
fn diagram_figure() -> Rule {
	Rule::new()
		.with_selector(Selector::class(DIAGRAM))
		.with_canonical(Display::Block)
		.with_token(common_props::BackgroundColor, diagram_surface()).unwrap()
		.with_token(common_props::ForegroundColor, colors::OnSurface).unwrap()
		.with_token(ShapeProps, geometry::ShapeSmall).unwrap()
		.with_value(common_props::Padding, Spacing::all(Length::Rem(1.)))
		.with_value(common_props::MarginProp, Spacing {
			bottom: Length::Rem(1.),
			..Spacing::DEFAULT
		})
		.with_value(common_props::MaxWidth, Length::Percent(100.))
		.with_value(common_props::OverflowXProp, Overflow::Auto)
}

/// `.diagram > svg` - a block filling the figure's width; the root keeps its
/// `viewBox` and no fixed size, so the browser derives the height, and a
/// one-off `max-width` on the root pins its natural width.
fn diagram_svg() -> Rule {
	Rule::new()
		.with_selector(Selector::child(Selector::class(DIAGRAM), Selector::tag("svg")))
		.with_canonical(Display::Block)
		.with_value(common_props::Width, Length::Percent(100.))
}

/// `.diagram-text` - mono, whitespace preserved and never wrapped; the figure
/// carries the padding and gap, so the inner `<pre>` drops its own.
fn diagram_text() -> Rule {
	Rule::new()
		.with_selector(Selector::class(DIAGRAM_TEXT))
		.with_token(common_props::FontFamilyProp, typography::TypefaceMono).unwrap()
		.with_canonical(WhiteSpace::Pre)
		.with_value(common_props::Padding, Spacing::DEFAULT)
		.with_value(common_props::MarginProp, Spacing::DEFAULT)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::style::material::MaterialStylePlugin;

	/// The served stylesheet declares every paint role on `:root` under the
	/// builder's shortened variable names, so an svg's `var(--diagram-*)` lands
	/// on the same declaration a `.btn` would.
	#[beet_core::test]
	fn paint_roles_reach_the_stylesheet() {
		let mut app = App::new();
		app.add_plugins((StylePlugin, MaterialStylePlugin));
		let css = app
			.world_mut()
			.run_system_once(|style: StyleQuery| {
				style
					.build_css(
						&CssBuilder::default()
							.with_format_variables(FormatVariables::short()),
					)
					.unwrap()
			})
			.unwrap();
		css.as_str()
			.xpect_contains("--diagram-node-fill: var(--material-colors-primary-container);")
			.xpect_contains("--diagram-font: var(--material-typography-typeface-plain);")
			.xpect_contains("--diagram-ramp-12-text: var(--material-colors-on-tertiary);")
			.xpect_contains("--diagram-render: auto;")
			.xpect_contains(".diagram > svg {");
		// declared again on each scheme class, where the redirect resolves to
		// that scheme's tones
		css.as_str()
			.split(".dark-scheme {")
			.nth(1)
			.unwrap()
			.split('}')
			.next()
			.unwrap()
			.xpect_contains("--diagram-node-fill: var(--material-colors-primary-container);");
	}

	/// A page root re-paints its diagrams with the same declaration surface
	/// `bx:style` gives every prop, landing in the stylesheet as the one-off
	/// rule the svg's `var(--diagram-node-fill)` then reads.
	#[beet_core::test]
	fn a_page_repaints_its_diagrams() {
		let (_, rule) =
			inline_style_rule("diagram-node-fill=@token:TertiaryContainer")
				.unwrap();
		CssRule::from_rule(&common_props::token_map(), &rule)
			.unwrap()
			.into_declarations()
			.into_iter()
			.map(|(key, value)| format!("{key}: {value}"))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"--diagram-node-fill: var(--io-crates-beet-ui-style-material-colors-tertiary-container)".to_string(),
			]);
	}
}
