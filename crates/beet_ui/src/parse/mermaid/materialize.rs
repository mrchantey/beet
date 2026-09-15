use crate::prelude::*;
use crate::render::tui_inset;
use crate::render::viewport_cells;
use crate::style::DiagramRender;
use crate::style::common_props::DiagramRenderProp;
use crate::style::common_props::Padding;
use beet_core::prelude::*;

/// Give every fresh [`MermaidDiagram`] figure (one with no children yet) its
/// form. The mode is `diagram-render` resolved through the cascade for the
/// figure; in this build every mode is text: a `<pre class="diagram-text">` of
/// box-drawing glyphs, reflowed to the surface's column budget on a terminal and
/// unbounded on the web. `Svg` warns that it degraded, never errors. A render
/// error keeps the source visible in a `<pre>` under a material error box, and
/// warns.
pub(crate) fn materialize_diagrams(
	mut commands: Commands,
	diagrams: Populated<(Entity, &MermaidDiagram), Without<Children>>,
	rules: RuleSetQuery,
	surfaces: SurfaceQuery,
	viewports: Query<&MediaViewport>,
) {
	let mut memo = CascadeMemo::default();
	for (figure, diagram) in diagrams.iter() {
		let render = rules
			.resolve(figure, DiagramRenderProp, &mut memo)
			.unwrap_or_default();
		if render == DiagramRender::Svg {
			warn!(
				"`diagram-render=Svg` needs the `mermaid_svg` feature, rendering `{}` as text",
				diagram.title()
			);
		}
		// a surface is a terminal: reflow to its columns. The web has none.
		let budget = surfaces
			.surface_of(figure)
			.and_then(|surface| viewports.get(surface).ok())
			.map(|viewport| column_budget(&rules, figure, viewport, &mut memo));
		match mermaid_text::render_with_width(&diagram.source, budget) {
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
fn spawn_error(
	commands: &mut Commands,
	figure: Entity,
	source: &str,
	err: mermaid_text::Error,
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

	#[beet_core::test]
	fn renders_text_on_the_string_sink() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		parse_md(app.world_mut(), root, FLOWCHART);
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, app.world_mut()))
			.unwrap()
			.to_string()
			.xpect_contains("<figure class=\"diagram\">")
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
