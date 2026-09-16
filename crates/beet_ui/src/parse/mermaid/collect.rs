use crate::prelude::*;
use crate::style::DiagramRender;
use crate::style::inline_style_rule;
use beet_core::prelude::*;

/// Walk every ```` ```mermaid ```` fence (`<pre><code class="mermaid">`, with or
/// without a `language-` prefix) and turn its `<pre>` into a
/// `<figure class="diagram">` carrying [`MermaidDiagram`], the `<code>`
/// despawned so the highlighter never sees it. The info word after `mermaid`
/// (`text | svg | auto`) becomes an inline `diagram-render` declaration on the
/// figure, the same one-off rule `bx:style` registers; any other word warns and
/// is ignored.
///
/// Idempotent: a figure has no `<code>` child, so a later run skips it.
pub(crate) fn collect_mermaid_blocks(
	mut commands: Commands,
	elements: ElementQuery,
	parents: Query<&ChildOf>,
) {
	for code in elements.iter().filter(is_mermaid_code) {
		let Some((_, value)) = code.inner_text else {
			continue;
		};
		let Ok(source) = value.as_str() else { continue };
		// a fence's `<code>` sits alone in a `<pre>`; an inline
		// `<code class="mermaid">` is prose and stays one
		let Some(figure) = parents
			.get(code.entity)
			.ok()
			.map(ChildOf::parent)
			.filter(|pre| {
				elements.get(*pre).is_ok_and(|view| view.tag() == "pre")
			})
		else {
			continue;
		};
		commands.entity(code.entity).despawn();
		commands.entity(figure).insert((
			Element::new("figure"),
			Classes::new([DIAGRAM]),
			MermaidDiagram::new(source),
		));
		// the info word after the language is this diagram's own mode
		let info = code.attribute_string("data-info");
		let Some(word) = info.split_whitespace().nth(1) else {
			continue;
		};
		match DiagramRender::parse_word(word) {
			Some(render) => {
				commands.entity(figure).queue(
					move |mut entity: EntityWorldMut| {
						declare_render(&mut entity, render)
					},
				);
			}
			None => warn!(
				"unknown mermaid fence info word `{word}`, expected `text`, `svg` or `auto`"
			),
		}
	}
}

/// A `<code>` whose class names the mermaid language.
fn is_mermaid_code(view: &ElementView) -> bool {
	view.tag() == "code"
		&& view.iter_classes().any(|class| {
			class.strip_prefix("language-").unwrap_or(&class) == "mermaid"
		})
}

/// Declare `render` on `entity` exactly as `bx:style="diagram-render=.."`
/// would: the content-keyed inline class and its one-off rule, so a fence info
/// word, a `<Mermaid render=..>` prop and the directive all share one rule.
pub(crate) fn declare_render(
	entity: &mut EntityWorldMut,
	render: DiagramRender,
) -> Result {
	let (class, rule) =
		inline_style_rule(&format!("diagram-render={render:?}"))?;
	register_inline_rule(entity, class, rule)
}

#[cfg(all(test, feature = "markdown_parser"))]
pub(super) mod test {
	use super::*;
	use crate::style::common_props::DiagramRenderProp;

	const FLOWCHART: &str = "```mermaid\ngraph LR; A[Start] --> B[End]\n```";

	/// Parse markdown under `root`, running the post-parse passes.
	pub(in crate::parse::mermaid) fn parse_md(
		world: &mut World,
		root: Entity,
		md: &str,
	) {
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut world.entity_mut(root),
				&MediaBytes::new_markdown(md),
			))
			.unwrap();
	}

	/// The `diagram-render` mode the cascade resolves for `entity`.
	fn resolve_render(world: &mut World, entity: Entity) -> DiagramRender {
		world.with_state::<RuleSetQuery, _>(|rules| {
			rules
				.resolve(entity, DiagramRenderProp, &mut CascadeMemo::default())
				.unwrap_or_default()
		})
	}

	/// The tags of every element under `root`, pre-order.
	fn tags(world: &mut World, root: Entity) -> Vec<String> {
		world.with_state::<ElementQuery, _>(|elements| {
			elements
				.iter_descendants_inclusive(root)
				.map(|view| view.tag().to_string())
				.collect()
		})
	}

	#[beet_core::test]
	fn fence_becomes_a_figure() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		parse_md(app.world_mut(), root, FLOWCHART);
		let world = app.world_mut();
		let figure = world.entity(root).get::<Children>().unwrap()[0];
		// the pre is now the figure, its code gone, the form beneath it
		let built = tags(world, root);
		built[0].as_str().xpect_eq("figure");
		built.iter().any(|tag| tag == "code").xpect_false();
		built.len().xpect_greater_than(1);
		let diagram = world.entity(figure).get::<MermaidDiagram>().unwrap();
		diagram.kind.xpect_eq(DiagramKind::Flowchart);
		diagram.source.as_str().xpect_contains("A[Start]");
		world
			.entity(figure)
			.get::<Classes>()
			.unwrap()
			.contains_name(&DIAGRAM)
			.xpect_true();
		// running the passes again leaves the figure alone
		world.run_schedule(PostParseTree);
		tags(world, root).xpect_eq(built);
	}

	#[beet_core::test]
	fn inline_mermaid_code_is_prose() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		parse_md(app.world_mut(), root, "use `graph LR`{.mermaid} here");
		tags(app.world_mut(), root)
			.iter()
			.any(|tag| tag == "figure")
			.xpect_false();
	}

	#[beet_core::test]
	fn info_word_overrides_the_page_mode() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		// the page asks for svg; the first fence insists on text
		let page = app
			.world_mut()
			.spawn((Element::new("div"), inline_class![(
				DiagramRenderProp,
				DiagramRender::Svg
			)]))
			.id();
		parse_md(
			app.world_mut(),
			page,
			&format!(
				"{}\n\n{FLOWCHART}",
				FLOWCHART.replace("mermaid", "mermaid text")
			),
		);
		let world = app.world_mut();
		let figures = world.entity(page).get::<Children>().unwrap().to_vec();
		figures.len().xpect_eq(2);
		resolve_render(world, figures[0]).xpect_eq(DiagramRender::Text);
		resolve_render(world, figures[1]).xpect_eq(DiagramRender::Svg);
	}

	#[beet_core::test]
	fn unknown_info_word_is_ignored() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		parse_md(
			app.world_mut(),
			root,
			&FLOWCHART.replace("mermaid", "mermaid banana"),
		);
		let world = app.world_mut();
		let figure = world.entity(root).get::<Children>().unwrap()[0];
		// no inline rule was declared, the mode is the default and the form still builds
		resolve_render(world, figure).xpect_eq(DiagramRender::Auto);
		world
			.entity(figure)
			.get::<Classes>()
			.unwrap()
			.len()
			.xpect_eq(1);
		tags(world, root).len().xpect_greater_than(1);
	}
}
