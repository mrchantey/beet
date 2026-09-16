//! The crate theme from the diagram paint roles: one field list, fed by a
//! paint source. The web build paints every role as its `var(--diagram-*)`
//! custom property so the picture follows the `.light-scheme`/`.dark-scheme`
//! body class with no re-render; the terminal build paints the same roles as
//! hex resolved through the cascade for the figure.
use crate::prelude::*;
#[cfg(any(feature = "tui", test))]
use crate::style::common_props::*;
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
		&mut |paint| paint.css_value(),
		ThemeMetrics::resolve(rules, figure, memo),
	)
}

/// The theme and layout for a terminal raster: every role resolved through the
/// cascade for `figure` to `#rrggbb` (alpha dropped, as the cell paint drops
/// it), so a `.dark-scheme` page rasterises in its dark tones. A role the
/// cascade has no value for (no material tokens registered) keeps the crate's
/// own colour, warned once per figure.
#[cfg(any(feature = "tui", test))]
pub(super) fn terminal_theme(
	rules: &RuleSetQuery,
	figure: Entity,
	memo: &mut CascadeMemo,
) -> (Theme, LayoutConfig) {
	let metrics = ThemeMetrics::resolve(rules, figure, memo);
	let ramp = rules.resolve(figure, DiagramRampProp, memo).ok();
	let mut missing = Vec::new();
	let theme = build_theme(
		&mut |paint| {
			resolve_paint(rules, figure, memo, ramp.as_ref(), paint)
				.unwrap_or_else(|| {
					missing.push(paint.css_name());
					crate_paint(paint)
				})
		},
		metrics,
	);
	if !missing.is_empty() {
		warn!(
			"diagram paint unresolved for the terminal, using the crate's colours: {}",
			missing.join(", ")
		);
	}
	theme
}

/// `paint` resolved for `figure`: a colour role as hex, the font as its css
/// family list, a ramp step through its `ColorRole` token pair.
#[cfg(any(feature = "tui", test))]
fn resolve_paint(
	rules: &RuleSetQuery,
	figure: Entity,
	memo: &mut CascadeMemo,
	ramp: Option<&DiagramRamp>,
	paint: DiagramPaint,
) -> Option<String> {
	use DiagramPaint::*;
	let mut hex = |token: Token| {
		rules
			.resolve_untyped(figure, &token, memo)
			.and_then(|value| value.into_serde::<Color>())
			.ok()
			.map(|color| {
				let [red, green, blue] =
					color.to_srgba().to_u8_array_no_alpha();
				format!("#{red:02x}{green:02x}{blue:02x}")
			})
	};
	match paint {
		Surface => hex(DiagramSurfaceProp.into()),
		NodeFill => hex(DiagramNodeFillProp.into()),
		NodeText => hex(DiagramNodeTextProp.into()),
		Outline => hex(DiagramOutlineProp.into()),
		Line => hex(DiagramLineProp.into()),
		Text => hex(DiagramTextProp.into()),
		SecondaryFill => hex(DiagramSecondaryFillProp.into()),
		TertiaryFill => hex(DiagramTertiaryFillProp.into()),
		TertiaryText => hex(DiagramTertiaryTextProp.into()),
		ClusterFill => hex(DiagramClusterFillProp.into()),
		ClusterOutline => hex(DiagramClusterOutlineProp.into()),
		RampFill(step) => hex(ramp?.step(step)?.background.clone()),
		RampText(step) => hex(ramp?.step(step)?.foreground.clone()),
		Font => rules
			.resolve(figure, DiagramFontProp, memo)
			.ok()
			.and_then(|typeface| typeface.as_css_value().ok())
			.map(|value| value.to_string()),
	}
}

/// The crate's own colour for `paint`, the field its default theme fills the
/// role from.
#[cfg(any(feature = "tui", test))]
fn crate_paint(paint: DiagramPaint) -> String {
	use DiagramPaint::*;
	let theme = Theme::mermaid_default();
	match paint {
		Surface => theme.background,
		NodeFill => theme.primary_color,
		NodeText => theme.primary_text_color,
		Outline => theme.primary_border_color,
		Line => theme.line_color,
		Text => theme.text_color,
		SecondaryFill => theme.secondary_color,
		TertiaryFill => theme.tertiary_color,
		TertiaryText => theme.git_tag_label_color,
		ClusterFill => theme.cluster_background,
		ClusterOutline => theme.cluster_border,
		Font => theme.font_family,
		RampFill(step) => {
			theme.pie_colors[step % theme.pie_colors.len()].clone()
		}
		RampText(step) => {
			theme.git_inv_colors[step % theme.git_inv_colors.len()].clone()
		}
	}
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
	paint: &mut dyn FnMut(DiagramPaint) -> String,
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
	use crate::style::material::MaterialStylePlugin;
	use crate::style::material::classes;

	/// Every colour and font field of the web theme is a paint `var()`: no
	/// baked hex anywhere the crate reads a colour, so the scheme class alone
	/// re-themes a picture.
	#[beet_core::test]
	fn web_theme_is_all_variables() {
		let (theme, layout) =
			build_theme(&mut |paint| paint.css_value(), default());
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

	/// The terminal theme paints every role as resolved hex, following the
	/// scheme class of the figure's page: a dark page's node fill is the dark
	/// `PrimaryContainer`, a light page's the light one, no `var()` anywhere.
	/// The scheme sits on a `<body>` below the document root, as the layout
	/// places it, so a `:root` redirect resolved at the root would miss it.
	#[beet_core::test]
	fn terminal_theme_resolves_hex_per_scheme() {
		let mut world = (StylePlugin, MaterialStylePlugin).into_world();
		let figure_under = |world: &mut World, scheme: ClassName| {
			let html = world
				.spawn((rsx! { <html/> }, children![(
					rsx! { <body/> },
					Classes::new([scheme]),
					children![(rsx! { <main/> }, children![
						rsx! { <figure/> }
					])]
				)]))
				.id();
			world.with_state::<ElementQuery, _>(|elements| {
				elements
					.iter_descendants_inclusive(html)
					.find(|view| view.tag() == "figure")
					.unwrap()
					.entity
			})
		};
		let light = figure_under(&mut world, classes::LIGHT_SCHEME);
		let dark = figure_under(&mut world, classes::DARK_SCHEME);
		let (light, dark) = world.with_state::<RuleSetQuery, _>(|rules| {
			let memo = &mut CascadeMemo::default();
			(
				terminal_theme(&rules, light, memo).0,
				terminal_theme(&rules, dark, memo).0,
			)
		});
		let is_hex = |value: &str| {
			value.len() == 7
				&& value.starts_with('#')
				&& value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
		};
		is_hex(&light.primary_color).xpect_true();
		is_hex(&dark.primary_color).xpect_true();
		(light.primary_color != dark.primary_color).xpect_true();
		is_hex(&dark.pie_colors[11]).xpect_true();
		is_hex(&dark.git_inv_colors[3]).xpect_true();
		dark.font_family
			.as_str()
			.xpect_contains("sans-serif")
			.xnot()
			.xpect_contains("var(");
	}

	/// Without material tokens the terminal theme keeps the crate's colours
	/// rather than failing the raster.
	#[beet_core::test]
	fn terminal_theme_falls_back_to_the_crate() {
		let mut world = StylePlugin::world();
		let figure = world.spawn(rsx! { <figure/> }).id();
		let (theme, _) = world.with_state::<RuleSetQuery, _>(|rules| {
			terminal_theme(&rules, figure, &mut CascadeMemo::default())
		});
		theme
			.primary_color
			.xpect_eq(Theme::mermaid_default().primary_color);
	}
}
