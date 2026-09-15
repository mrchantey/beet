#![cfg_attr(rustfmt, rustfmt_skip)]
//! The diagram classes and the rules that style them, colocated with the
//! passes that emit them.
use crate::prelude::*;
use crate::style::common_props;
use crate::style::material::*;
use crate::style::*;

/// The figure a diagram renders into, on the code-block surface.
pub const DIAGRAM: ClassName = ClassName::new_static("diagram");
/// The `<pre>` holding a diagram's box-drawing text.
pub const DIAGRAM_TEXT: ClassName = ClassName::new_static("diagram-text");

/// The rules styling [`DIAGRAM`] and [`DIAGRAM_TEXT`] on both sinks.
pub(crate) fn diagram_rules() -> Vec<Rule> {
	vec![diagram_figure(), diagram_text()]
}

/// `.diagram` - the `pre` surface (fill, padding, corner), so a diagram and a
/// code block on one page share a look, a block gap below, and a horizontal
/// scroll on the web for art wider than the column.
fn diagram_figure() -> Rule {
	Rule::new()
		.with_selector(Selector::class(DIAGRAM))
		.with_canonical(Display::Block)
		.with_token(common_props::BackgroundColor, colors::SurfaceContainerHighest).unwrap()
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
