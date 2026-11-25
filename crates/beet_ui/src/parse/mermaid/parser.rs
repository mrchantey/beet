//! [`MermaidParser`], a mermaid document as the figure that draws it.
use crate::prelude::*;
use crate::style::DiagramRender;
use crate::style::InlineStyle;
use crate::style::inline_style_rule;
use beet_core::prelude::*;
use beet_scene::prelude::*;

/// Reads mermaid as a `<figure>` holding its [`MermaidDiagram`], drawn once it
/// is on a surface: a ```` ```mermaid ```` fence, and a document of its own.
///
/// The word after `mermaid` in a fence's info string
/// (`text | svg | auto`, [`FenceInfo`]) is an [`InlineStyle`] declaring
/// `diagram-render`, as `bx:style` would; any other word warns and is ignored.
#[derive(Debug, Default, Clone)]
pub struct MermaidParser;

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

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError> {
		let mut figure = EntityBody::default()
			.with_typed(&Element::new("figure"))?
			.with_typed(&MermaidDiagram::new(bytes.as_utf8()?))?;
		if let Some(word) = FenceInfo::of(parsers).split_whitespace().next() {
			match DiagramRender::parse_word(word) {
				Some(render) => figure
					.insert_typed(&InlineStyle::new(render.declaration()))?,
				None => warn!(
					"unknown mermaid fence info word `{word}`, expected `text`, `svg` or `auto`"
				),
			}
		}
		ParsedDocument::from(SceneBody::root(figure)).xok()
	}
}

impl Installer for MermaidParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

/// Declare `render` on `entity` exactly as `bx:style="diagram-render=.."`
/// would: the content-keyed inline class and its one-off rule, so a fence info
/// word, a `<Mermaid render=..>` prop and the directive all share one rule.
pub(crate) fn declare_render(
	entity: &mut EntityWorldMut,
	render: DiagramRender,
) -> Result {
	let (class, rule) = inline_style_rule(&render.declaration())?;
	register_inline_rule(entity, class, rule)
}

#[cfg(all(test, feature = "markdown_parser"))]
pub(super) mod test {
	use super::*;
	use crate::style::common_props::DiagramRenderProp;

	const FLOWCHART: &str = "```mermaid\ngraph LR; A[Start] --> B[End]\n```";

	/// Parse markdown under `root` through the media parse, which runs the
	/// post-parse passes.
	pub(in crate::parse::mermaid) fn parse_md(
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
