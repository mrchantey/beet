//! [`MermaidParser`], a mermaid document as the figure that draws it.
use crate::prelude::*;
use crate::style::DiagramRender;
use crate::style::InlineStyle;
use beet_core::prelude::*;
use beet_scene::prelude::*;

/// Reads mermaid as a `<figure>` holding its [`MermaidDiagram`], drawn once it
/// is on a surface: a ```` ```mermaid ```` fence, which renders unless it says
/// `render=false`, and a document of its own.
///
/// A fence's `mode` (`text | svg | auto`, [`MermaidFence`]) is an
/// [`InlineStyle`] declaring `diagram-render`, as `bx:style` would.
#[derive(Debug, Default, Clone)]
pub struct MermaidParser {
	/// What the fence being read says, nothing for a document of its own.
	fence: MermaidFence,
}

/// What a ```` ```mermaid ```` fence says beyond its format, ie
/// ```` ```mermaid mode=text ````.
#[derive(Debug, Default, Clone, PartialEq, Reflect)]
pub struct MermaidFence {
	/// The form this diagram takes, else the cascade's `diagram-render`.
	pub mode: Option<DiagramRender>,
}

impl MermaidParser {
	/// Mermaid's media type, and the word a fence names it by.
	const MEDIA_TYPES: [&'static str; 2] = ["text/vnd.mermaid", "mermaid"];
}

impl DocParser for MermaidParser {
	fn media_types(&self) -> Vec<MediaType> {
		Self::MEDIA_TYPES
			.map(|name| MediaType::Other(SmolStr::new_static(name)))
			.to_vec()
	}

	/// A diagram is drawn wherever it is written.
	fn renders_fence(&self) -> bool { true }

	fn configure(&mut self, params: &FenceParams) -> Result {
		self.fence = params.read()?;
		Ok(())
	}

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		_parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError> {
		let mut figure = EntityBody::default()
			.with_typed(&Element::new("figure"))?
			.with_typed(&MermaidDiagram::new(bytes.as_utf8()?))?;
		if let Some(mode) = self.fence.mode {
			figure.insert_typed(&InlineStyle::new(mode.declaration()))?;
		}
		ParsedDocument::from(SceneBody::root(figure)).xok()
	}
}

impl Installer for MermaidParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

#[cfg(all(test, feature = "markdown"))]
pub(super) mod test {
	use super::*;
	use crate::style::common_props::DiagramRenderProp;

	const FLOWCHART: &str = "```mermaid\ngraph LR; A[Start] --> B[End]\n```";

	/// Parse markdown under `root` through the media parse, which runs the
	/// post-parse passes.
	pub(in crate::formats::mermaid) fn parse_md(
		world: &mut World,
		root: Entity,
		md: &str,
	) {
		ParseContext::new(
			&mut world.entity_mut(root),
			&MediaBytes::new_markdown(md),
		)
		.parse()
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
	fn fence_mode_overrides_the_page_mode() {
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
				FLOWCHART.replace("mermaid", "mermaid mode=text")
			),
		);
		let world = app.world_mut();
		let figures = world.entity(page).get::<Children>().unwrap().to_vec();
		figures.len().xpect_eq(2);
		resolve_render(world, figures[0]).xpect_eq(DiagramRender::Text);
		resolve_render(world, figures[1]).xpect_eq(DiagramRender::Svg);
	}

	#[beet_core::test]
	fn unknown_mode_refuses_the_page() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		ParseContext::new(
			&mut app.world_mut().entity_mut(root),
			&MediaBytes::new_markdown(
				&FLOWCHART.replace("mermaid", "mermaid mode=banana"),
			),
		)
		.parse()
		.unwrap_err()
		.to_string()
		.xpect_contains("banana");
	}

	#[beet_core::test]
	fn render_false_is_a_code_sample() {
		let mut app = App::new();
		app.add_plugins(StylePlugin);
		let root = app.world_mut().spawn_empty().id();
		parse_md(
			app.world_mut(),
			root,
			&FLOWCHART.replace("mermaid", "mermaid render=false"),
		);
		let built = tags(app.world_mut(), root);
		built.iter().any(|tag| tag == "figure").xpect_false();
		built.iter().any(|tag| tag == "code").xpect_true();
	}
}
