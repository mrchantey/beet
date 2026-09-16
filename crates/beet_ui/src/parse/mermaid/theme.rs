//! The crate theme from the diagram paint roles: one field list, fed by a
//! paint source. The web build paints every role as its `var(--diagram-*)`
//! custom property so the picture follows the `.light-scheme`/`.dark-scheme`
//! body class with no re-render; a terminal build paints the same roles as
//! resolved hex.
use crate::prelude::*;
use crate::style::material::geometry::*;
use crate::style::material::typography::*;
use crate::style::*;
use beet_core::prelude::*;
use mermaid_rs_renderer::LayoutConfig;
use mermaid_rs_renderer::Theme;

/// The theme and layout for the web: every role a `var()`, the numbers
/// resolved through the cascade for `figure`.
pub(super) fn web_theme(
	rules: &RuleSetQuery,
	figure: Entity,
	memo: &mut CascadeMemo,
) -> (Theme, LayoutConfig) {
	build_theme(
		&|paint| paint.css_value(),
		ThemeMetrics::resolve(rules, figure, memo),
	)
}

/// The numbers a theme carries that no `var()` can: the crate measures text
/// and draws corners and strokes with them, so every sink resolves them through
/// the cascade for the figure. A missing token keeps the material default.
#[derive(Debug, Clone, Copy)]
pub(super) struct ThemeMetrics {
	/// Node and edge labels, `FontSizeBodyMedium`.
	pub font_size: f32,
	/// Diagram titles, `FontSizeTitleLarge`.
	pub title_size: f32,
	/// Pie sections and legends, `FontSizeBodyLarge`.
	pub label_size: f32,
	/// `ShapeCornerSmall`, where the crate exposes a radius.
	pub corner_radius: f32,
	/// `OutlineWidthThin`, where the crate exposes a stroke width.
	pub outline_width: f32,
}

impl Default for ThemeMetrics {
	fn default() -> Self {
		Self {
			font_size: 14.,
			title_size: 22.,
			label_size: 16.,
			corner_radius: 8.,
			outline_width: 1.,
		}
	}
}

impl ThemeMetrics {
	/// Resolve each length for `figure`, in pixels.
	pub fn resolve(
		rules: &RuleSetQuery,
		figure: Entity,
		memo: &mut CascadeMemo,
	) -> Self {
		let defaults = Self::default();
		let mut px = |token: Token, default: f32| {
			rules
				.resolve_untyped(figure, &token, memo)
				.and_then(|value| value.into_serde::<Length>())
				.map(|length| length.into_px(Vec2::ZERO))
				.unwrap_or(default)
		};
		Self {
			font_size: px(FontSizeBodyMedium.into(), defaults.font_size),
			title_size: px(FontSizeTitleLarge.into(), defaults.title_size),
			label_size: px(FontSizeBodyLarge.into(), defaults.label_size),
			corner_radius: px(ShapeCornerSmall.into(), defaults.corner_radius),
			outline_width: px(OutlineWidthThin.into(), defaults.outline_width),
		}
	}
}

/// The theme and layout from a paint source and the resolved metrics: the
/// role table in one place, every field named so a crate update that adds one
/// is a decision here rather than a stray default.
pub(super) fn build_theme(
	paint: &dyn Fn(DiagramPaint) -> String,
	metrics: ThemeMetrics,
) -> (Theme, LayoutConfig) {
	use DiagramPaint::*;
	let fills = std::array::from_fn(|step| paint(RampFill(step)));
	let ons: [String; DiagramRamp::STEPS] =
		std::array::from_fn(|step| paint(RampText(step)));
	let theme = Theme {
		font_family: paint(Font),
		font_size: metrics.font_size,
		primary_color: paint(NodeFill),
		primary_text_color: paint(NodeText),
		primary_border_color: paint(Outline),
		line_color: paint(Line),
		secondary_color: paint(SecondaryFill),
		tertiary_color: paint(TertiaryFill),
		edge_label_background: paint(Surface),
		cluster_background: paint(ClusterFill),
		cluster_border: paint(ClusterOutline),
		background: paint(Surface),
		sequence_actor_fill: paint(NodeFill),
		sequence_actor_border: paint(Outline),
		sequence_actor_line: paint(ClusterOutline),
		sequence_note_fill: paint(TertiaryFill),
		sequence_note_border: paint(Outline),
		sequence_activation_fill: paint(SecondaryFill),
		sequence_activation_border: paint(Outline),
		text_color: paint(Text),
		git_colors: first_n(&fills),
		git_inv_colors: first_n(&ons),
		git_branch_label_colors: first_n(&ons),
		git_commit_label_color: paint(Text),
		git_commit_label_background: paint(ClusterFill),
		git_tag_label_color: paint(TertiaryText),
		git_tag_label_background: paint(TertiaryFill),
		git_tag_label_border: paint(Outline),
		pie_colors: fills,
		pie_title_text_size: metrics.title_size,
		pie_title_text_color: paint(Text),
		pie_section_text_size: metrics.label_size,
		pie_section_text_color: paint(Text),
		pie_legend_text_size: metrics.label_size,
		pie_legend_text_color: paint(Text),
		// slices separate on the surface, the pie outlines like a node
		pie_stroke_color: paint(Surface),
		pie_stroke_width: metrics.outline_width,
		pie_outer_stroke_width: metrics.outline_width,
		pie_outer_stroke_color: paint(Outline),
		// material containers are already tonal
		pie_opacity: 1.,
	};
	let mut layout = LayoutConfig {
		fast_text_metrics: true,
		..default()
	};
	// every root carries `width`/`height`: the responsive sizing is the figure's
	// (`.diagram > svg`), not the crate's `use_max_width` inline style
	layout.pie.use_max_width = false;
	layout.mindmap.use_max_width = false;
	layout.gitgraph.use_max_width = false;
	layout.c4.use_max_width = false;
	layout.mindmap.default_corner_radius = metrics.corner_radius;
	layout.gitgraph.branch_label_corner_radius = metrics.corner_radius / 2.;
	let requirement = &mut layout.requirement;
	requirement.fill = paint(NodeFill);
	requirement.box_stroke = paint(Outline);
	requirement.box_stroke_width = metrics.outline_width;
	requirement.stroke = paint(Outline);
	requirement.stroke_width = metrics.outline_width;
	requirement.label_color = paint(NodeText);
	requirement.divider_color = paint(ClusterOutline);
	requirement.divider_width = metrics.outline_width;
	requirement.edge_stroke = paint(Line);
	requirement.edge_stroke_width = metrics.outline_width;
	requirement.edge_label_color = paint(Text);
	requirement.edge_label_background = paint(Surface);
	(theme, layout)
}

/// The first `N` steps of the ramp.
fn first_n<const N: usize>(ramp: &[String; DiagramRamp::STEPS]) -> [String; N] {
	std::array::from_fn(|step| ramp[step].clone())
}

#[cfg(test)]
mod test {
	use super::*;

	/// Every colour and font field of the web theme is a paint `var()`: no
	/// baked hex anywhere the crate reads a colour, so the scheme class alone
	/// re-themes a picture.
	#[beet_core::test]
	fn web_theme_is_all_variables() {
		let (theme, layout) =
			build_theme(&|paint| paint.css_value(), default());
		serde_json::to_value(&theme)
			.unwrap()
			.as_object()
			.unwrap()
			.iter()
			.flat_map(|(key, value)| match value {
				serde_json::Value::String(value) => vec![(key, value.clone())],
				serde_json::Value::Array(values) => values
					.iter()
					.map(|value| (key, value.as_str().unwrap().to_string()))
					.collect(),
				_ => vec![],
			})
			.filter(|(_, value)| !value.starts_with("var(--diagram-"))
			.map(|(key, value)| format!("{key}: {value}"))
			.collect::<Vec<_>>()
			.xpect_eq(Vec::<String>::new());
		theme.background.xpect_eq("var(--diagram-surface)");
		theme.font_family.xpect_eq("var(--diagram-font)");
		theme.pie_colors[11].xpect_eq("var(--diagram-ramp-12-fill)");
		layout.fast_text_metrics.xpect_true();
		layout.pie.use_max_width.xpect_false();
		layout.mindmap.default_corner_radius.xpect_eq(8.);
	}
}
