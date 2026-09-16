//! The picture forms: the crate's svg, themed by `var()` tokens so it follows
//! the colour scheme live and parsed into the figure as an inline `<svg>` on
//! the web, or themed by resolved hex and rasterised onto an `<img>` beneath
//! the figure for a graphics terminal; either way rendered once per source
//! and theme through [`DiagramSvgCache`].
use super::materialize::spawn_error;
#[cfg(feature = "tui")]
use super::theme::terminal_theme;
use super::theme::web_theme;
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use mermaid_rs_renderer::LayoutConfig;
use mermaid_rs_renderer::Theme;
use std::hash::BuildHasher;
use std::sync::Arc;

/// Rendered svg by source and resolved theme, so a page built again (a live
/// reload, the next request for it) lays out only the diagrams whose source or
/// paint changed. The web theme is one per page (`var()` strings and the
/// figure's metrics) and the terminal theme one per scheme, so a repainted
/// page or a dark session is its own entry. Bounded at [`Self::CAP`] entries
/// by starting over, a generation rather than an eviction order: a server's
/// hits are the pages it serves and a dev session's the diagram under edit,
/// neither helped by ordering.
#[derive(Default, Resource)]
pub(crate) struct DiagramSvgCache(HashMap<u64, Picture>);

/// A rendered svg and its natural width.
#[derive(Clone)]
struct Picture {
	svg: Arc<str>,
	width: f32,
}

impl DiagramSvgCache {
	/// The entries kept before the cache starts over.
	const CAP: usize = 512;

	/// The picture for `source` under `theme` and `layout`: cached, or laid
	/// out now and kept. A source the crate rejects is never kept, so it
	/// reports its error on every build.
	fn render(
		&mut self,
		source: &str,
		theme: &Theme,
		layout: &LayoutConfig,
	) -> Result<Picture> {
		let key = FixedHasher.hash_one((
			source,
			format!("{theme:?}"),
			format!("{layout:?}"),
		));
		if let Some(picture) = self.0.get(&key) {
			return picture.clone().xok();
		}
		let (svg, width) = render(source, theme, layout)?;
		let picture = Picture {
			svg: svg.into(),
			width,
		};
		if self.0.len() >= Self::CAP {
			self.0.clear();
		}
		self.0.insert(key, picture.clone());
		picture.xok()
	}

	/// The pictures kept.
	#[cfg(test)]
	fn len(&self) -> usize { self.0.len() }
}

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
	cache: &mut DiagramSvgCache,
) {
	let (theme, layout) = web_theme(rules, figure, memo);
	match cache.render(&diagram.source, &theme, &layout) {
		Ok(Picture { svg, width }) => {
			commands
				.entity(figure)
				.queue(move |mut entity: EntityWorldMut| {
					spawn_inline_svg(&mut entity, &svg, width)
				});
		}
		Err(err) => spawn_error(commands, figure, &diagram.source, err),
	}
}

/// Render `diagram` as terminal-themed svg and rasterise it as kitty image
/// `id` onto an `<img>` beneath `figure`, alt-texted with the diagram's title:
/// the picture is a replaced box sized by the raster and contained by the
/// figure, so a small diagram stays small as the web pins its natural width,
/// and a wide one shrinks to the column. The raster lands asynchronously
/// (resvg runs on the blocking pool), the `<img>` showing its alt marker until
/// then, and on a raster failure the marker beside the material error box, as
/// any `<img>` does. The svg string is never spawned as entities on a
/// terminal. A parse error keeps the source visible under the error box, and
/// warns.
#[cfg(feature = "tui")]
pub(super) fn spawn_raster(
	commands: &mut Commands,
	figure: Entity,
	diagram: &MermaidDiagram,
	rules: &RuleSetQuery,
	memo: &mut CascadeMemo,
	cache: &mut DiagramSvgCache,
	id: u32,
) {
	let (theme, layout) = terminal_theme(rules, figure, memo);
	match cache.render(&diagram.source, &theme, &layout) {
		Ok(Picture { svg, .. }) => {
			let title = diagram.title().to_string();
			let subject: SmolStr = format!("mermaid diagram `{title}`").into();
			commands
				.spawn((rsx! { <img alt=title/> }, ChildOf(figure)))
				.queue_async(move |entity| {
					attach_raster(
						entity,
						Ok(svg.as_bytes().to_vec()),
						id,
						subject,
					)
				});
		}
		Err(err) => spawn_error(commands, figure, &diagram.source, err),
	}
}

/// The svg string and its natural width, in one parse and layout; the cache
/// above is the way in for a build.
fn render(
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

	/// A page built again lays out only the diagrams that changed: one source
	/// under one theme is one picture however many figures show it, a changed
	/// source or a page whose metrics resolve differently another.
	#[beet_core::test]
	fn caches_by_source_and_theme() {
		use crate::style::material::typography::FontSizeBodyMedium;

		let mut world = StylePlugin.into_world();
		// a fresh page each time, as a reload or a request builds one
		let build = |world: &mut World, page: Entity, md: &str| {
			parse_md(world, page, md);
			world.resource::<DiagramSvgCache>().len()
		};
		let page = |world: &mut World| world.spawn_empty().id();
		let fresh = page(&mut world);
		build(&mut world, fresh, FLOWCHART).xpect_eq(1);
		// the same fence twice on another page: the one picture
		let fresh = page(&mut world);
		build(&mut world, fresh, &format!("{FLOWCHART}\n\n{FLOWCHART}"))
			.xpect_eq(1);
		let fresh = page(&mut world);
		build(&mut world, fresh, &FLOWCHART.replace("Paint", "Draw"))
			.xpect_eq(2);
		// a page whose labels resolve to another size lays out afresh
		let larger = world
			.spawn((Element::new("div"), inline_class![(
				FontSizeBodyMedium,
				Length::Px(20.)
			)]))
			.id();
		build(&mut world, larger, FLOWCHART).xpect_eq(3);
		// a parse error is never kept
		let fresh = page(&mut world);
		build(&mut world, fresh, "```mermaid\ngraph LR\n--> A\n```")
			.xpect_eq(3);
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
		use crate::render::decode_image;
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
		let image = decode_image(svg.into_bytes(), 1).unwrap();
		image.px.x.xpect_eq((width * 2.).ceil() as u32);
		image.px.y.xpect_greater_than(100);
		// sized at its css width, as the web pins it
		image
			.cell_size(CellBounds::new(1000, 1000))
			.x
			.xpect_eq((width.ceil() as u32).div_ceil(10));
	}
}
