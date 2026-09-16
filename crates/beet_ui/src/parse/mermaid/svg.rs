//! The picture forms: the crate's svg, themed by `var()` tokens so it follows
//! the colour scheme live and parsed into the figure as an inline `<svg>` on
//! the web, or themed by resolved hex and rasterised onto the figure for a
//! graphics terminal.
use super::materialize::spawn_error;
#[cfg(feature = "tui")]
use super::theme::terminal_theme;
use super::theme::web_theme;
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use mermaid_rs_renderer::LayoutConfig;
use mermaid_rs_renderer::Theme;

/// Render `diagram` as web-themed svg and spawn it beneath `figure` as an
/// inline `<svg>`, sized responsively: the root's `width` and `height` go, the
/// `viewBox` stays, and a content-keyed `max-width` pins the natural width so a
/// small diagram stays small while a wide one shrinks to the column
/// (`.diagram > svg` is `width: 100%`). A parse error keeps the source visible
/// under a material error box, and warns.
pub(super) fn spawn_svg(
	commands: &mut Commands,
	figure: Entity,
	diagram: &MermaidDiagram,
	rules: &RuleSetQuery,
	memo: &mut CascadeMemo,
) {
	let (theme, layout) = web_theme(rules, figure, memo);
	match render(&diagram.source, &theme, &layout) {
		Ok((svg, width)) => {
			commands
				.entity(figure)
				.queue(move |mut entity: EntityWorldMut| {
					spawn_inline_svg(&mut entity, &svg, width)
				});
		}
		Err(err) => spawn_error(commands, figure, &diagram.source, err),
	}
}

/// Render `diagram` as terminal-themed svg and rasterise it onto `figure` as
/// kitty image `id`: the figure itself carries the [`KittyImage`], so the
/// measure pass sizes it as a replaced box and `place_kitty_images` draws it;
/// the svg string is never spawned as entities on a terminal. The raster lands
/// asynchronously (resvg runs on the blocking pool), the figure empty until
/// then; a raster failure marks it [`KittyImageUnavailable`], the material
/// error box under the figure. A parse error keeps the source visible under
/// the error box, and warns.
#[cfg(feature = "tui")]
pub(super) fn spawn_raster(
	commands: &mut Commands,
	figure: Entity,
	diagram: &MermaidDiagram,
	rules: &RuleSetQuery,
	memo: &mut CascadeMemo,
	id: u32,
) {
	let (theme, layout) = terminal_theme(rules, figure, memo);
	match render(&diagram.source, &theme, &layout) {
		Ok((svg, _)) => {
			let subject: SmolStr =
				format!("mermaid diagram `{}`", diagram.title()).into();
			commands.entity(figure).queue_async(move |entity| {
				attach_raster(entity, Ok(svg.into_bytes()), id, subject)
			});
		}
		Err(err) => spawn_error(commands, figure, &diagram.source, err),
	}
}

/// The svg string and its natural width, in one parse and layout.
pub(super) fn render(
	source: &str,
	theme: &Theme,
	layout: &LayoutConfig,
) -> Result<(String, f32)> {
	let parsed = mermaid_rs_renderer::parse_mermaid_strict(source)?;
	let computed =
		mermaid_rs_renderer::compute_layout(&parsed.graph, theme, layout);
	let width =
		mermaid_rs_renderer::measure_svg_dimensions(&computed, layout, None)
			.width;
	let svg = mermaid_rs_renderer::render_svg(&computed, theme, layout);
	Ok((svg, width))
}

/// Parse `svg` into `figure`'s children, then make the root responsive.
fn spawn_inline_svg(
	figure: &mut EntityWorldMut,
	svg: &str,
	width: f32,
) -> Result {
	BsxParser::html()
		.parse(ParseContext::new(figure, &MediaBytes::new_html(svg)))?;
	let figure_id = figure.id();
	figure.world_scope(|world| {
		let root = world
			.get::<Children>(figure_id)
			.into_iter()
			.flat_map(|children| children.iter())
			.find(|child| {
				world
					.get::<Element>(*child)
					.is_some_and(|el| el.tag() == "svg")
			})
			.ok_or_else(|| bevyhow!("the rendered svg has no `<svg>` root"))?;
		make_responsive(world, root, width)
	})
}

/// Drop the root's `width` and `height` so the stylesheet sizes it from the
/// `viewBox`, and pin its natural width as the one-off rule
/// `bx:style="max-width=Px(..)"` would declare.
fn make_responsive(world: &mut World, root: Entity, width: f32) -> Result {
	let sized: Vec<Entity> = world
		.get::<Attributes>(root)
		.into_iter()
		.flat_map(|attrs| attrs.iter())
		.filter(|attr| {
			world
				.get::<Attribute>(*attr)
				.is_some_and(|key| matches!(key.as_str(), "width" | "height"))
		})
		.collect();
	for attr in sized {
		world.despawn(attr);
	}
	let class =
		ClassName::from_inline_source(&format!("max-width=Px({width})"));
	let rule = Rule::new()
		.with_selector(Selector::Class(class.as_selector()))
		.with_value(common_props::MaxWidth, Length::Px(width));
	register_inline_rule(&mut world.entity_mut(root), class, rule)
}

#[cfg(all(test, feature = "markdown_parser"))]
mod test {
	use super::*;
	use crate::parse::mermaid::collect::test::parse_md;
	use crate::style::DiagramRender;
	use crate::style::common_props::DiagramRenderProp;

	const FLOWCHART: &str =
		"```mermaid\ngraph LR; A[Parse] --> B[Style]; B --> C[Paint]\n```";

	fn render_md(md: &str) -> String {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		parse_md(app.world_mut(), root, md);
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, app.world_mut()))
			.unwrap()
			.to_string()
	}

	/// `Auto` on the web is the picture: an inline `<svg>` whose colours are
	/// the token variables, its `viewBox` intact and its fixed size gone.
	#[beet_core::test]
	fn renders_svg_on_the_web() {
		let html = render_md(FLOWCHART);
		html.xref()
			.xpect_contains("<figure class=\"diagram\"><svg ")
			.xpect_contains(" viewBox=\"0 0 ")
			.xpect_contains("fill=\"var(--diagram-node-fill)\"")
			.xpect_contains("stroke=\"var(--diagram-line)\"")
			.xpect_contains("font-family=\"var(--diagram-font)\"")
			.xpect_contains("Paint")
			.xnot()
			.xpect_contains("#0F172A")
			.xnot()
			.xpect_contains("diagram-text");
		// the root keeps no fixed size, only the pinned natural width
		let root = html
			.split("<svg ")
			.nth(1)
			.unwrap()
			.split('>')
			.next()
			.unwrap();
		root.xref()
			.xpect_contains("class=\"")
			.xnot()
			.xpect_contains(" width=")
			.xnot()
			.xpect_contains(" height=");
		html.xpect_snapshot();
	}

	/// The `Svg` mode is the picture too, and the `%%{init}%%` directive the
	/// crate parses still renders.
	#[beet_core::test]
	fn honours_svg_mode_and_init_directive() {
		render_md(
			"```mermaid svg\n%%{init: {\"theme\": \"dark\"}}%%\nsequenceDiagram\nAlice->>Bob: hi\n```",
		)
		.xpect_contains("<svg ")
		.xpect_contains("Alice");
	}

	/// The page-level `Text` still wins over the web default.
	#[beet_core::test]
	fn text_mode_stays_text() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let page = app
			.world_mut()
			.spawn((Element::new("div"), inline_class![(
				DiagramRenderProp,
				DiagramRender::Text
			)]))
			.id();
		parse_md(app.world_mut(), page, FLOWCHART);
		HtmlRenderer::new()
			.render(&mut RenderContext::new(page, app.world_mut()))
			.unwrap()
			.to_string()
			.xpect_contains("diagram-text")
			.xnot()
			.xpect_contains("<svg");
	}

	/// A diagram the crate rejects keeps its source under the error box.
	#[beet_core::test]
	fn parse_error_keeps_the_source() {
		render_md("```mermaid\ngraph LR\n--> A\n```")
			.xpect_contains("<div class=\"error\">mermaid: ")
			.xpect_contains("<pre>graph LR")
			.xnot()
			.xpect_contains("<svg");
	}

	/// The terminal-themed svg of a sequence diagram rasterises through the
	/// kitty path to a valid PNG at 2x its natural size, its colours the
	/// resolved hex.
	#[cfg(feature = "tui")]
	#[beet_core::test]
	fn rasterizes_a_sequence_diagram() {
		use crate::render::encode_png;
		use crate::render::to_png_bytes;
		use crate::style::material::MaterialStylePlugin;

		let mut world = (StylePlugin, MaterialStylePlugin).into_world();
		let figure = world.spawn(rsx! { <figure/> }).id();
		let (theme, layout) = world.with_state::<RuleSetQuery, _>(|rules| {
			terminal_theme(&rules, figure, &mut CascadeMemo::default())
		});
		let (svg, width) =
			render("sequenceDiagram\nAlice->>Bob: hi", &theme, &layout)
				.unwrap();
		// the material palette, not the crate's default
		(theme.sequence_actor_fill
			!= Theme::mermaid_default().sequence_actor_fill)
			.xpect_true();
		svg.as_str()
			.xpect_contains(&theme.sequence_actor_fill)
			.xnot()
			.xpect_contains("var(");
		let px = to_png_bytes(svg.into_bytes())
			.and_then(encode_png)
			.unwrap()
			.1;
		px.x.xpect_eq((width * 2.).ceil() as u32);
		px.y.xpect_greater_than(100);
	}
}
