use crate::prelude::*;
use crate::render::tui_inset;
use crate::render::viewport_cells;
use crate::style::DiagramRender;
use crate::style::common_props::DiagramRenderProp;
use crate::style::common_props::Padding;
use beet_core::prelude::*;

/// The sink and width a figure's form was built for, so a later pass on
/// another sink rebuilds it: a served page is built with no surface (the web's
/// picture) and then painted for a terminal by the one-shot ansi renderer,
/// which runs the passes again under its buffer; a live terminal resize
/// changes the column budget.
#[derive(Debug, Clone, PartialEq, Component)]
pub(crate) struct DiagramForm {
	/// The mode the cascade resolved for the figure.
	render: DiagramRender,
	/// The terminal's column budget, `None` on the web.
	columns: Option<usize>,
}

/// Give every [`MermaidDiagram`] figure its form, rebuilding one whose sink,
/// width or mode ([`DiagramForm`]) changed since it was built. The mode is
/// `diagram-render` resolved through the cascade for the figure:
///
/// - On the web (no surface viewport above the figure) `Auto` and `Svg` are
///   the crate's picture, spawned inline (`svg.rs`). A build without
///   `mermaid_svg` warns that `Svg` degraded, never errors.
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
) {
	let mut memo = CascadeMemo::default();
	for (figure, diagram, built) in diagrams.iter() {
		let render = rules
			.resolve(figure, DiagramRenderProp, &mut memo)
			.unwrap_or_default();
		// a viewport above the figure is a terminal; the web has none
		let columns = rules.surface_viewport(figure).map(|viewport| {
			column_budget(&rules, figure, &viewport, &mut memo)
		});
		let form = DiagramForm { render, columns };
		if built == Some(&form) {
			continue;
		}
		// a form built for another sink or width goes, this one replaces it
		if built.is_some() {
			commands.entity(figure).despawn_related::<Children>();
		}
		commands.entity(figure).insert(form);
		if columns.is_none() && render != DiagramRender::Text {
			#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
			{
				super::svg::spawn_svg(
					&mut commands,
					figure,
					diagram,
					&rules,
					&mut memo,
				);
				continue;
			}
			#[cfg(not(all(
				feature = "mermaid_svg",
				not(target_arch = "wasm32")
			)))]
			if render == DiagramRender::Svg {
				warn!(
					"`diagram-render=Svg` needs the `mermaid_svg` feature, rendering `{}` as text",
					diagram.title()
				);
			}
		}
		match mermaid_text::render_with_width(&diagram.source, columns) {
			Ok(text) => spawn_text(&mut commands, figure, text),
			Err(err) => {
				spawn_error(&mut commands, figure, &diagram.source, err)
			}
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
}
