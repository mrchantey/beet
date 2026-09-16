use crate::prelude::*;
use crate::render::tui_inset;
use crate::render::viewport_cells;
use crate::style::DiagramRender;
use crate::style::common_props::DiagramRenderProp;
use crate::style::common_props::Padding;
use beet_core::prelude::*;

/// Whether this build renders the svg form: the `mermaid_svg` feature, native
/// only.
const SVG: bool =
	cfg!(all(feature = "mermaid_svg", not(target_arch = "wasm32")));
/// Whether this build rasterises a picture for a graphics terminal: the svg
/// renderer plus the `tui` raster path.
const RASTER: bool = SVG && cfg!(feature = "tui");

/// The form built beneath a [`MermaidDiagram`] figure, recorded so a later pass
/// whose target differs rebuilds it: a served page is built with no surface
/// (the web's picture) and then painted for a terminal by the one-shot ansi
/// renderer, which runs the passes again under its buffer; a live terminal
/// resize changes the text's column budget; a session's graphics support
/// arrives with its terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub(crate) enum DiagramForm {
	/// The crate's picture as an inline `<svg>`, the web.
	Svg,
	/// Box-drawing text in a `<pre class="diagram-text">`, reflowed to a
	/// terminal's column budget, unbounded on the web.
	Text { columns: Option<usize> },
	/// The picture rasterised onto the figure itself as a [`KittyImage`], a
	/// graphics terminal.
	Raster,
}

impl DiagramForm {
	/// The form `render` takes on `sink` for a diagram of `kind`, the `Auto`
	/// rule: the picture where the sink shows one, an inline svg on the web
	/// and a raster on a graphics terminal for every type but a flowchart,
	/// which reflows as text; text everywhere else. `Svg` on a terminal
	/// without graphics is text too: no picture can show there. A build without
	/// a renderer never targets its form, so `Svg` and `Raster` are only ever
	/// built where they can be.
	fn target(render: DiagramRender, kind: DiagramKind, sink: Sink) -> Self {
		match (render, sink) {
			(DiagramRender::Text, Sink::Web) => Self::Text { columns: None },
			(_, Sink::Web) if SVG => Self::Svg,
			(_, Sink::Web) => Self::Text { columns: None },
			(DiagramRender::Text, Sink::Terminal { columns, .. }) => {
				Self::Text {
					columns: Some(columns),
				}
			}
			(DiagramRender::Auto, Sink::Terminal { columns, .. })
				if kind == DiagramKind::Flowchart =>
			{
				Self::Text {
					columns: Some(columns),
				}
			}
			(_, Sink::Terminal { graphics: true, .. }) if RASTER => {
				Self::Raster
			}
			(_, Sink::Terminal { columns, .. }) => Self::Text {
				columns: Some(columns),
			},
		}
	}
}

/// Where a figure renders: the web (no surface viewport above it), or a
/// terminal surface with its column budget and whether its terminal draws
/// kitty graphics.
#[derive(Debug, Clone, Copy)]
enum Sink {
	Web,
	Terminal { columns: usize, graphics: bool },
}

/// Give every [`MermaidDiagram`] figure its form, rebuilding one whose target
/// ([`DiagramForm::target`]) differs from what was built. The mode is
/// `diagram-render` resolved through the cascade for the figure; the sink is
/// the surface viewport above it (a terminal buffer carries one, the web
/// none), whose entity also carries the terminal's [`KittyGraphicsSupport`].
///
/// - The web: `Auto` and `Svg` are the crate's picture, spawned inline
///   (`svg.rs`). A build without `mermaid_svg` warns that `Svg` degraded,
///   never errors.
/// - A graphics terminal: `Auto` for a flowchart is text (it reflows to the
///   columns), `Auto` for every other type and `Svg` a raster on the figure.
/// - Everywhere else, and for `Text`, a `<pre class="diagram-text">` of
///   box-drawing glyphs, reflowed to the surface's column budget on a terminal
///   and unbounded on the web.
///
/// A render error keeps the source visible in a `<pre>` under a material error
/// box, and warns.
pub(crate) fn materialize_diagrams(
	mut commands: Commands,
	diagrams: Populated<(Entity, &MermaidDiagram, Option<&DiagramForm>)>,
	rules: RuleSetQuery,
	#[cfg(all(
		feature = "tui",
		feature = "mermaid_svg",
		not(target_arch = "wasm32")
	))]
	support: Query<&KittyGraphicsSupport>,
	// the raster's id comes from the placement state, which only a charcell
	// world carries (a web build's style-only world has none)
	#[cfg(all(
		feature = "tui",
		feature = "mermaid_svg",
		not(target_arch = "wasm32")
	))]
	mut placements: Option<ResMut<KittyPlacements>>,
) {
	let mut memo = CascadeMemo::default();
	for (figure, diagram, built) in diagrams.iter() {
		let render = rules
			.resolve(figure, DiagramRenderProp, &mut memo)
			.unwrap_or_default();
		let sink = match rules.surface_viewport(figure) {
			None => Sink::Web,
			#[allow(unused_variables)]
			Some((surface, viewport)) => Sink::Terminal {
				columns: column_budget(&rules, figure, &viewport, &mut memo),
				#[cfg(all(
					feature = "tui",
					feature = "mermaid_svg",
					not(target_arch = "wasm32")
				))]
				graphics: placements.is_some()
					&& support
						.get(surface)
						.is_ok_and(|support| support.enabled),
				#[cfg(not(all(
					feature = "tui",
					feature = "mermaid_svg",
					not(target_arch = "wasm32")
				)))]
				graphics: false,
			},
		};
		let form = DiagramForm::target(render, diagram.kind, sink);
		if built == Some(&form) {
			continue;
		}
		// a form built for another sink, width or mode goes, this one replaces it
		if built.is_some() {
			commands
				.entity(figure)
				.despawn_related::<Children>()
				.queue(|mut entity: EntityWorldMut| detach_image(&mut entity));
		}
		commands.entity(figure).insert(form);
		if render == DiagramRender::Svg && !SVG {
			warn!(
				"`diagram-render=Svg` needs the `mermaid_svg` feature, rendering `{}` as text",
				diagram.title()
			);
		}
		match form {
			DiagramForm::Text { columns } => {
				match mermaid_text::render_with_width(&diagram.source, columns)
				{
					Ok(text) => spawn_text(&mut commands, figure, text),
					Err(err) => {
						spawn_error(&mut commands, figure, &diagram.source, err)
					}
				}
			}
			#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
			DiagramForm::Svg => super::svg::spawn_svg(
				&mut commands,
				figure,
				diagram,
				&rules,
				&mut memo,
			),
			#[cfg(all(
				feature = "tui",
				feature = "mermaid_svg",
				not(target_arch = "wasm32")
			))]
			DiagramForm::Raster => super::svg::spawn_raster(
				&mut commands,
				figure,
				diagram,
				&rules,
				&mut memo,
				placements.as_mut().unwrap().alloc_id(),
			),
			// a form this build has no renderer for is never targeted
			#[allow(unreachable_patterns)]
			_ => {}
		}
	}
}

/// The columns a text diagram may take on a terminal: the surface's width less
/// the figure's own horizontal inset, so the art fits inside its padded box.
fn column_budget(
	rules: &RuleSetQuery,
	figure: Entity,
	viewport: &MediaViewport,
	memo: &mut CascadeMemo,
) -> usize {
	let cells = viewport_cells(viewport);
	let inset = rules
		.resolve(figure, Padding, memo)
		.map(|padding| tui_inset(&padding, cells.as_vec2()))
		.unwrap_or_default();
	cells.x.saturating_sub(inset.min.x + inset.max.x) as usize
}

/// `<pre class="diagram-text">` holding the rendered art.
fn spawn_text(commands: &mut Commands, figure: Entity, text: String) {
	commands.spawn((
		Element::new("pre"),
		Classes::new([DIAGRAM_TEXT]),
		ChildOf(figure),
		children![Value::str(text)],
	));
}

/// The material error box naming the failure, then the source in a `<pre>` so
/// the author still sees what they wrote.
pub(super) fn spawn_error(
	commands: &mut Commands,
	figure: Entity,
	source: &str,
	err: impl std::fmt::Display,
) {
	warn!("mermaid diagram failed to render: {err}\n{source}");
	commands.spawn((
		Element::new("div"),
		Classes::new([classes::ERROR]),
		ChildOf(figure),
		children![Value::str(format!("mermaid: {err}"))],
	));
	commands.spawn((Element::new("pre"), ChildOf(figure), children![
		Value::str(source)
	]));
}

#[cfg(all(test, feature = "markdown_parser"))]
mod test {
	use super::*;
	use crate::parse::mermaid::collect::test::parse_md;
	use crate::render::CharcellPlugin;
	use crate::render::FlexBuffer;

	const FLOWCHART: &str =
		"```mermaid\ngraph LR; A[Parse] --> B[Style]; B --> C[Paint]\n```";

	/// The `text` info word is the box art on the web too.
	#[beet_core::test]
	fn renders_text_on_the_string_sink() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		parse_md(
			app.world_mut(),
			root,
			&FLOWCHART.replace("mermaid", "mermaid text"),
		);
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, app.world_mut()))
			.unwrap()
			.to_string()
			.xpect_contains("<figure class=\"diagram inline-style-")
			.xpect_contains("<pre class=\"diagram-text\">")
			.xpect_contains("Paint")
			.xpect_snapshot();
	}

	#[beet_core::test]
	fn renders_text_in_cells() {
		let mut world = CharcellPlugin::world();
		let root = world
			.spawn((FlexBuffer::new(40), RenderSurface::self_referential()))
			.id();
		parse_md(&mut world, root, FLOWCHART);
		let out = world
			.entity_mut(root)
			.take::<FlexBuffer>()
			.unwrap()
			.render_plain();
		// box-drawing glyphs land in cells, inside the figure's padding
		out.xpect_contains("─")
			.xpect_contains("Paint")
			.xmap(|out| {
				out.lines()
					.map(|line| line.trim_end())
					.filter(|line| !line.is_empty())
					.collect::<Vec<_>>()
					.join("\n")
			})
			.xpect_snapshot();
	}

	/// A served page is built with no surface, then the one-shot ansi renderer
	/// paints it under a buffer: the form built for the web is rebuilt as text
	/// for the terminal, and a narrower buffer reflows it.
	#[beet_core::test]
	fn rebuilds_the_form_for_the_sink() {
		let mut world = CharcellPlugin::world();
		let root = world.spawn_empty().id();
		parse_md(&mut world, root, FLOWCHART);
		let first_form = |world: &mut World| {
			world.with_state::<ElementQuery, _>(|elements| {
				elements
					.iter_descendants_inclusive(root)
					.nth(1)
					.map(|view| view.tag().to_string())
			})
		};
		let rendered = |world: &mut World, width: u32| {
			world.entity_mut(root).insert(FlexBuffer::new(width));
			world.run_schedule(PostParseTree);
			world
				.entity_mut(root)
				.take::<FlexBuffer>()
				.unwrap()
				.render_plain()
		};
		#[cfg(feature = "mermaid_svg")]
		first_form(&mut world).xpect_eq(Some("svg".to_string()));
		let wide = rendered(&mut world, 80);
		first_form(&mut world).xpect_eq(Some("pre".to_string()));
		wide.xref().xpect_contains("│ Parse │────");
		// a narrower terminal reflows the art, a repeat at the same width keeps it
		let narrow = rendered(&mut world, 40);
		narrow.xref().xnot().xpect_contains("│ Parse │────");
		let figure = world.entity(root).get::<Children>().unwrap()[0];
		let art = world.entity(figure).get::<Children>().unwrap().to_vec();
		rendered(&mut world, 40).xpect_eq(narrow);
		world
			.entity(figure)
			.get::<Children>()
			.unwrap()
			.to_vec()
			.xpect_eq(art);
	}

	#[beet_core::test]
	fn render_error_keeps_the_source() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		parse_md(app.world_mut(), root, "```mermaid\nnotadiagram\n```");
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, app.world_mut()))
			.unwrap()
			.to_string()
			.xpect_contains("<div class=\"error\">mermaid: ")
			.xpect_contains("<pre>notadiagram")
			.xnot()
			.xpect_contains("diagram-text");
	}

	/// A build with no renderer for a form never targets it.
	#[beet_core::test]
	fn target_names_only_renderable_forms() {
		let terminal = |graphics| Sink::Terminal {
			columns: 60,
			graphics,
		};
		let text = |columns| DiagramForm::Text { columns };
		let target = DiagramForm::target;
		// the web: the picture where the build renders one, else text
		target(DiagramRender::Auto, DiagramKind::Other, Sink::Web)
			.xpect_eq(if SVG { DiagramForm::Svg } else { text(None) });
		target(DiagramRender::Text, DiagramKind::Other, Sink::Web)
			.xpect_eq(text(None));
		// a graphics terminal: a flowchart reflows, the rest rasterise
		target(DiagramRender::Auto, DiagramKind::Flowchart, terminal(true))
			.xpect_eq(text(Some(60)));
		target(DiagramRender::Auto, DiagramKind::Other, terminal(true))
			.xpect_eq(if RASTER {
				DiagramForm::Raster
			} else {
				text(Some(60))
			});
		target(DiagramRender::Svg, DiagramKind::Flowchart, terminal(true))
			.xpect_eq(if RASTER {
				DiagramForm::Raster
			} else {
				text(Some(60))
			});
		// no graphics: text, whatever was asked
		target(DiagramRender::Svg, DiagramKind::Other, terminal(false))
			.xpect_eq(text(Some(60)));
		target(DiagramRender::Text, DiagramKind::Other, terminal(true))
			.xpect_eq(text(Some(60)));
	}
}

/// The `Auto` rule on a live terminal, driven through the tui host so the
/// raster lands through the real async attach.
#[cfg(all(
	test,
	feature = "markdown_parser",
	feature = "tui",
	feature = "mermaid_svg"
))]
mod raster_test {
	use super::*;
	use crate::parse::mermaid::collect::test::parse_md;
	use crate::render::test_host::TestHost;
	use bevy::math::UVec2;

	const PAGE: &str = "```mermaid\ngraph LR; A[Parse] --> B[Paint]\n```\n\n```mermaid\nsequenceDiagram\nAlice->>Bob: hi\n```";

	/// An 80x24 host whose terminal reports `support`, showing [`PAGE`]: a
	/// flowchart then a sequence diagram, both `Auto`.
	fn diagram_host(support: KittyGraphicsSupport) -> TestHost {
		let mut host = TestHost::sized(UVec2::new(80, 24));
		// the raster attaches from an async task
		host.app.init_plugin::<AsyncPlugin>();
		host.app.world_mut().entity_mut(host.host).insert(support);
		parse_md(host.app.world_mut(), host.host, PAGE);
		host.step();
		host
	}

	/// The figures under the host, in order.
	fn figures(host: &TestHost) -> Vec<Entity> {
		host.app
			.world()
			.entity(host.host)
			.get::<Children>()
			.unwrap()
			.to_vec()
	}

	/// The tag of `figure`'s first child, `None` when it has no children (a
	/// raster form carries the picture on the figure itself).
	fn first_child_tag(host: &mut TestHost, figure: Entity) -> Option<String> {
		host.app
			.world_mut()
			.with_state::<ElementQuery, _>(|elements| {
				elements
					.iter_descendants_inclusive(figure)
					.nth(1)
					.map(|view| view.tag().to_string())
			})
	}

	/// Drive the host until the sequence figure carries its raster.
	async fn settle_raster(host: &mut TestHost) -> Entity {
		let sequence = figures(host)[1];
		app_ext::update_until_timeout(
			&mut host.app,
			|world| world.entity(sequence).contains::<KittyImage>(),
			Duration::from_secs(60),
		)
		.await
		.xpect_true();
		host.step();
		sequence
	}

	/// On a graphics terminal `Auto` reflows the flowchart as text and
	/// rasterises the sequence diagram onto its figure, transmitted as a kitty
	/// image; a `from_pty` session detecting the same support behaves alike.
	#[beet_core::test]
	async fn graphics_terminal_rasterizes_all_but_flowcharts() {
		for support in [
			KittyGraphicsSupport { enabled: true },
			KittyGraphicsSupport::from_pty(
				"xterm-256color",
				UVec2::new(1666, 2170),
			),
		] {
			let mut host = diagram_host(support);
			let figures = figures(&host);
			let (flowchart, sequence) = (figures[0], figures[1]);
			first_child_tag(&mut host, flowchart).xpect_eq(Some("pre".into()));
			host.app
				.world()
				.entity(sequence)
				.get::<DiagramForm>()
				.unwrap()
				.xpect_eq(DiagramForm::Raster);
			settle_raster(&mut host).await;
			first_child_tag(&mut host, sequence).xpect_eq(None);
			let image = host
				.app
				.world()
				.entity(sequence)
				.get::<KittyImage>()
				.unwrap()
				.clone();
			image.px.x.xpect_greater_than(100);
			String::from_utf8_lossy(&host.frame_ansi())
				.into_owned()
				.xpect_contains(&format!(
					"\u{1b}_Ga=t,f=100,q=2,i={}",
					image.id
				))
				.xpect_contains(&format!("a=p,i={}", image.id));
			host.frame_plain().xpect_contains("│ Parse │");
		}
	}

	/// Without graphics every diagram is text.
	#[beet_core::test]
	fn plain_terminal_renders_text() {
		let mut host = diagram_host(KittyGraphicsSupport { enabled: false });
		for figure in figures(&host) {
			first_child_tag(&mut host, figure).xpect_eq(Some("pre".into()));
			host.app
				.world()
				.entity(figure)
				.contains::<KittyImage>()
				.xpect_false();
		}
		host.frame_plain()
			.as_str()
			.xpect_contains("│ Parse │")
			.xpect_contains("Alice");
	}

	/// A session losing its graphics rebuilds the raster figure as text,
	/// dropping the picture it carried.
	#[beet_core::test]
	async fn losing_graphics_rebuilds_as_text() {
		let mut host = diagram_host(KittyGraphicsSupport { enabled: true });
		let sequence = settle_raster(&mut host).await;
		host.app
			.world_mut()
			.entity_mut(host.host)
			.insert(KittyGraphicsSupport { enabled: false });
		host.step();
		host.app
			.world()
			.entity(sequence)
			.contains::<KittyImage>()
			.xpect_false();
		first_child_tag(&mut host, sequence).xpect_eq(Some("pre".into()));
		host.frame_plain().xpect_contains("Alice");
	}
}
