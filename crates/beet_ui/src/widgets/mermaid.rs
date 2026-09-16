//! The `<Mermaid>` widget: a diagram authored in markup rather than a fence,
//! its source the default slot's text or a file.
//!
//! The widget spawns the same `<figure class="diagram">` carrying
//! [`MermaidDiagram`] a fence collects into, so `materialize_diagrams` treats
//! both alike. Its source arrives after the build: the slot's text once slots
//! resolve ([`Ready`]), a file once its read lands, so the figure materialises
//! inline when the source arrives, as `CodeSnippet` highlights inline. The
//! post-parse pass may already have run (the read landed after it) or never
//! run at all (a BSX page on the string sink); either way the figure has its
//! form, and a terminal's pass rebuilds it for its sink as it does a fence.

use crate::prelude::*;
use crate::style::DiagramRender;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A mermaid diagram authored in markup. The source is the default slot's
/// text, or the file `src` names read through the nearest self-or-ancestor
/// [`BlobStore`] (a `.mmd` beside the page, never the filesystem directly).
/// `render` pins the form as a fence info word does; unset, the figure takes
/// the cascade's `diagram-render`.
///
/// ```html
/// <Mermaid render="Text">graph LR; A[Parse] --> B[Paint]</Mermaid>
/// <Mermaid src="./pipeline.mmd"/>
/// ```
///
/// A brace in BSX text opens an expression, so a diagram naming a
/// `B{Decision}` node is authored through `src` or a fence.
#[template]
pub fn Mermaid(
	/// A file holding the source, resolved through the nearest ancestor store;
	/// the default slot's text when empty.
	src: String,
	/// The form this diagram takes, the declaration a fence info word makes.
	render: Option<DiagramRender>,
) -> impl Bundle {
	rsx! {
		<figure
			{Classes::new([DIAGRAM])}
			{MermaidSource::new(src, render)}
		><Slot/></figure>
	}
}

/// Where a `<Mermaid>` figure's source comes from, adopted on [`Ready`]: the
/// build has settled and its slots resolved, so the slot's text sits under
/// the figure and the store an `src` reads through is reachable by ancestry.
#[derive(Debug, Component)]
#[component(on_add = hook_ext::observe(adopt_source))]
struct MermaidSource {
	/// The store path of the source, `None` for the slot's text.
	src: Option<SmolStr>,
	/// The form to declare on the figure, `None` for the cascade's.
	render: Option<DiagramRender>,
}

impl MermaidSource {
	fn new(src: String, render: Option<DiagramRender>) -> Self {
		Self {
			src: (!src.is_empty()).then(|| src.into()),
			render,
		}
	}
}

/// Observer: give the figure its source. The slot's text is under the figure
/// now; a file is read through the nearest self-or-ancestor [`BlobStore`] off
/// a task of the figure, cancelled with it.
fn adopt_source(
	ev: On<Ready>,
	sources: Query<&MermaidSource>,
	children: Query<&Children>,
	values: Query<&Value>,
	mut commands: Commands,
) {
	let figure = ev.entity;
	let Ok(source) = sources.get(figure) else {
		return;
	};
	let render = source.render;
	match source.src.clone() {
		None => {
			let text: String = children
				.get(figure)
				.into_iter()
				.flatten()
				.filter_map(|child| values.get(*child).ok()?.as_str().ok())
				.collect();
			commands
				.entity(figure)
				.queue(move |mut entity: EntityWorldMut| {
					materialize(&mut entity, text, render)
				});
		}
		Some(src) => {
			commands.entity(figure).queue_async_local(
				move |figure| async move {
					let store = figure
						.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
							|entity, stores| stores.get(entity).cloned(),
						)
						.await??;
					let bytes = store.get(&RelPath::from(src.as_str())).await?;
					let text = String::from_utf8(bytes.to_vec())?;
					figure
						.with(move |mut entity| {
							materialize(&mut entity, text, render)
						})
						.await?
				},
			);
		}
	}
}

/// Declare `render`, replace whatever the figure held (the slot's text, an
/// unfilled slot) with [`MermaidDiagram`], and build its form now.
fn materialize(
	figure: &mut EntityWorldMut,
	source: String,
	render: Option<DiagramRender>,
) -> Result {
	if let Some(render) = render {
		declare_render(figure, render)?;
	}
	figure
		.remove::<MermaidSource>()
		.despawn_related::<Children>()
		.insert(MermaidDiagram::new(source));
	figure
		.world_scope(|world| world.run_system_cached(materialize_diagrams))?;
	Ok(())
}

#[cfg(all(test, feature = "markdown_parser"))]
mod test {
	use super::*;
	use crate::style::common_props::DiagramRenderProp;

	/// Build `bsx` under a fresh root in a styled world, settling any read.
	async fn spawn(store: Option<BlobStore>, bsx: &str) -> (World, Entity) {
		let mut world = (AsyncPlugin, StylePlugin).into_world();
		let root = world.spawn_empty().id();
		if let Some(store) = store {
			world.entity_mut(root).insert(store);
		}
		BsxParser::bsx()
			.parse(ParseContext::new(
				&mut world.entity_mut(root),
				&MediaBytes::new_bsx(bsx),
			))
			.unwrap();
		AsyncRunner::settle_async_tasks(&mut world).await;
		(world, root)
	}

	fn html(world: &mut World, root: Entity) -> String {
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, world))
			.unwrap()
			.to_string()
	}

	fn figure(world: &World, root: Entity) -> Entity {
		world.entity(root).get::<Children>().unwrap()[0]
	}

	/// The `diagram-render` mode the cascade resolves for `entity`.
	fn resolve_render(world: &mut World, entity: Entity) -> DiagramRender {
		world.with_state::<RuleSetQuery, _>(|rules| {
			rules
				.resolve(entity, DiagramRenderProp, &mut CascadeMemo::default())
				.unwrap_or_default()
		})
	}

	/// The slot's text is the source, gone from the tree once adopted, and the
	/// figure has its form without any post-parse pass having run.
	#[beet_core::test]
	async fn slot_text_is_the_source() {
		let (mut world, root) = spawn(
			None,
			"<Mermaid render=\"Text\">\ngraph LR\n  A[Parse] --> B[Paint]\n</Mermaid>",
		)
		.await;
		let figure = figure(&world, root);
		world
			.entity(figure)
			.get::<MermaidDiagram>()
			.unwrap()
			.source
			.as_str()
			.xpect_contains("A[Parse] --> B[Paint]");
		world
			.entity(figure)
			.contains::<MermaidSource>()
			.xpect_false();
		resolve_render(&mut world, figure).xpect_eq(DiagramRender::Text);
		html(&mut world, root)
			.xpect_contains("<figure class=\"diagram inline-style-")
			.xpect_contains("<pre class=\"diagram-text\">")
			.xpect_contains("│ Parse │")
			.xnot()
			.xpect_contains("graph LR");
	}

	/// `src` reads through the ancestor store, and the unset `render` leaves
	/// the mode to the cascade.
	#[beet_core::test]
	async fn src_reads_the_ancestor_store() {
		let store = BlobStore::temp();
		store
			.insert(
				&RelPath::from("flow.mmd"),
				"sequenceDiagram\nAlice->>Bob: hi\n",
			)
			.await
			.unwrap();
		let (mut world, root) =
			spawn(Some(store), "<Mermaid src=\"flow.mmd\"/>").await;
		let figure = figure(&world, root);
		world
			.entity(figure)
			.get::<MermaidDiagram>()
			.unwrap()
			.title()
			.xpect_eq("sequenceDiagram");
		resolve_render(&mut world, figure).xpect_eq(DiagramRender::Auto);
		let html = html(&mut world, root);
		html.xref().xpect_contains("<figure class=\"diagram\">");
		#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
		html.xref().xpect_contains("<svg ");
		html.xpect_contains("Alice");
	}

	/// The docs page, `site/routes/docs/design/diagrams.md`: every fence and
	/// the `<Mermaid src>` include take a form on both sinks, none an error
	/// box. The web snapshot is each figure's form, the terminal snapshot the
	/// page's cells at 100 columns. The web forms are the svg renderer's, so
	/// native only.
	#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn docs_page_renders_on_both_sinks() {
		let Ok(routes) = AbsPath::new_workspace_rel("site/routes") else {
			return;
		};
		let md = fs_ext::read_to_string(routes.join("docs/design/diagrams.md"))
			.unwrap();
		let mut world = (AsyncPlugin, CharcellPlugin).into_world();
		let root = world.spawn(BlobStore::new(FsStore::new(routes))).id();
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut world.entity_mut(root),
				&MediaBytes::new_markdown(&md),
			))
			.unwrap();
		// the include reads its file off a task
		AsyncRunner::settle_async_tasks(&mut world).await;
		let forms = figure_forms(&mut world, root);
		forms.lines().count().xpect_eq(10);
		forms.xref().xnot().xpect_contains("error");
		forms.xpect_snapshot();
		world.entity_mut(root).insert(FlexBuffer::new(100));
		world.run_schedule(PostParseTree);
		let cells = world
			.entity_mut(root)
			.take::<FlexBuffer>()
			.unwrap()
			.render_plain()
			.lines()
			.map(str::trim_end)
			.filter(|line| !line.is_empty())
			.collect::<Vec<_>>()
			.join("\n");
		cells.xref().xnot().xpect_contains("mermaid:");
		cells.xpect_snapshot();
	}

	/// One line per `<figure>` under `root`, in document order: its classes,
	/// then each child's tag and classes, an svg's `viewBox` and an error's
	/// text, so a snapshot names every diagram's form without its markup.
	#[cfg(all(feature = "mermaid_svg", not(target_arch = "wasm32")))]
	fn figure_forms(world: &mut World, root: Entity) -> String {
		world.with_state::<(ElementQuery, Query<&Children>), _>(
			|(elements, children)| {
				let describe = |view: &ElementView| {
					let mut out = view.tag().to_string();
					for class in view.iter_classes() {
						out.push('.');
						out.push_str(&class);
					}
					if let Some(attr) = view.attribute("viewBox") {
						out.push_str(&format!("[viewBox={}]", attr.value));
					}
					if let Some((_, text)) = view
						.inner_text
						.filter(|_| view.contains_class_name(&classes::ERROR))
					{
						out.push_str(&format!(" {text:?}"));
					}
					out
				};
				elements
					.iter_descendants_inclusive(root)
					.filter(|view| view.tag() == "figure")
					.map(|figure| {
						children
							.get(figure.entity)
							.into_iter()
							.flatten()
							.filter_map(|child| elements.get(*child).ok())
							.map(|child| describe(&child))
							.collect::<Vec<_>>()
							.join(" + ")
							.xmap(|forms| {
								format!("{} > {forms}", describe(&figure))
							})
					})
					.collect::<Vec<_>>()
					.join("\n")
			},
		)
	}

	/// The markdown path: a `<Mermaid>` in a page builds through the diff, and
	/// its inline form matches what the post-parse pass would build, so the
	/// pass leaves it alone.
	#[beet_core::test]
	fn markdown_page_keeps_the_inline_form() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		MarkdownParser::new()
			.parse(ParseContext::new(
				&mut app.world_mut().entity_mut(root),
				&MediaBytes::new_markdown(
					"# Flow\n\n<Mermaid render=\"Text\">graph LR; A --> B</Mermaid>\n",
				),
			))
			.unwrap();
		let world = app.world_mut();
		let figure = world.entity(root).get::<Children>().unwrap()[1];
		let art = world.entity(figure).get::<Children>().unwrap().to_vec();
		world.run_schedule(PostParseTree);
		world
			.entity(figure)
			.get::<Children>()
			.unwrap()
			.to_vec()
			.xpect_eq(art);
		html(world, root).xpect_contains("<pre class=\"diagram-text\">");
	}
}
