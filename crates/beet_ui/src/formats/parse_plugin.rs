//! The formats this crate adds, and scheduling for the post-parse pipeline.
//!
//! `beet_scene`'s `ScenePlugin` registers the base formats, which
//! [`ParsePlugin`] installs (`scene`). The Office formats (`ooxml`) are
//! `OoxmlPlugin`'s, which a router installs: their writers resolve looks
//! through the style plugin, itself built over this one.
//!
//! [`PostParseTree`] is the single schedule where a parsed entity tree is
//! styled, decorated, laid out, and painted. Every render path runs it
//! on demand: `ParseContext::parse` and the charcell renderers call
//! [`World::run_schedule`]/[`World::try_run_schedule`] right after building
//! their tree.
//!
//! [`ParsePlugin`] (pulled in by [`StylePlugin`](crate::prelude::StylePlugin))
//! registers the schedule but deliberately does **not** wire it into the main
//! schedule order. A request/response server or a one-shot render must not
//! repaint every frame: doing so re-scans every parsed tree still resident in
//! the world on every tick, so latency climbs as routes accumulate their cached
//! trees. Only a realtime app driven by the main loop wants per-frame repaint,
//! and it opts in with [`RealtimeParsePlugin`].
use crate::prelude::*;
use beet_core::prelude::*;
#[cfg(feature = "scene")]
use beet_scene::prelude::*;

/// Installs the document formats and the [`PostParseTree`] schedule for
/// on-demand runs.
///
/// Does not add the schedule to the main schedule order, see the module docs.
/// Realtime apps add [`RealtimeParsePlugin`] for per-frame repaint.
#[derive(Default)]
pub struct ParsePlugin;

impl ParsePlugin {
	/// The base formats, declaring [`DocMeta`], for a caller with no app, ie a
	/// codegen scan.
	#[cfg(feature = "scene")]
	pub fn parsers() -> DocParsers {
		ScenePlugin::parsers()
			.with_metadata_type(TypeName::of_path::<DocMeta>())
	}
}

impl Plugin for ParsePlugin {
	fn build(&self, app: &mut App) {
		// ensure the schedule exists for on-demand `run_schedule` callers, without
		// adding it to the per-frame main loop (which would repaint the whole world
		// every tick).
		app.init_schedule(PostParseTree);
		// the base formats, a document's unsectioned metadata its `DocMeta`
		#[cfg(feature = "scene")]
		app.install_types::<ScenePlugin>()
			.world_mut()
			.get_resource_or_init::<DocParsers>()
			.set_metadata_type(TypeName::of_path::<DocMeta>());
		// the default BSX event vocabulary, so every `bx:onclick="..."` runs
		// its script on a click.
		#[cfg(feature = "scene")]
		app.install_types::<crate::prelude::BsxDefaultsPlugin>();
		// a document's root metadata and its route's, which frontmatter
		// declares and writes
		app.register_type::<DocMeta>()
			.register_type::<RouteMeta>()
			.register_type::<InlineBlob>();
		// the limits and vocabulary `beet check` holds every page's metadata to
		app.register_type::<DocMetaRules>()
			.register_type::<TextLimit>();
	}
}

/// Opt-in plugin for realtime apps driven by the main loop: runs the
/// [`PostParseTree`] pipeline after every [`Update`] so style changes and
/// document mutations repaint each frame.
///
/// A server or one-shot render must not add this — they run [`PostParseTree`]
/// on demand per request/render instead. See the module docs.
#[derive(Default)]
pub struct RealtimeParsePlugin;

impl Plugin for RealtimeParsePlugin {
	fn build(&self, app: &mut App) {
		app.install_types::<ParsePlugin>()
			.install(InsertSchedule::after(Update, PostParseTree));
	}
}

#[cfg(test)]
mod test {
	use super::*;
	#[cfg(feature = "markdown")]
	use beet_net::prelude::*;
	use bevy::app::MainScheduleOrder;
	use bevy::ecs::schedule::ScheduleLabel;

	fn in_main_order(app: &App) -> bool {
		let label = PostParseTree.intern();
		app.world()
			.resource::<MainScheduleOrder>()
			.labels
			.contains(&label)
	}

	/// The server/one-shot path must not repaint every frame: a request handler
	/// runs [`PostParseTree`] on demand, and leaving it in the main loop re-scans
	/// every parsed route tree each tick, so latency climbs as routes accumulate.
	#[beet_core::test]
	fn parse_plugin_stays_out_of_main_loop() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ParsePlugin));
		in_main_order(&app).xpect_false();
	}

	/// Realtime apps opt into per-frame repaint.
	#[beet_core::test]
	fn realtime_plugin_runs_after_update() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, RealtimeParsePlugin));
		in_main_order(&app).xpect_true();
	}

	/// Parse `bytes` onto a fresh entity of a world with this crate's
	/// parsers.
	fn parse(bytes: &MediaBytes) -> (World, Entity) {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		ParseContext::new(&mut world.entity_mut(entity), bytes)
			.parse()
			.unwrap();
		(world, entity)
	}

	/// The tag of `entity`.
	fn tag(world: &World, entity: Entity) -> &str {
		world.get::<Element>(entity).unwrap().tag()
	}

	/// Plain text lands on the entity it parses onto, the body's one entity.
	#[beet_core::test]
	fn parse_plain_text() {
		let (world, entity) = parse(&MediaBytes::new_text("hello"));
		world
			.get::<Value>(entity)
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("hello".into()));
	}

	/// `MediaType::Html` reads through the BSX parser, HTML being the
	/// features-off subset.
	#[cfg(feature = "scene")]
	#[beet_core::test]
	fn parse_html() {
		let (world, entity) = parse(&MediaBytes::new_html("<div>hello</div>"));
		tag(&world, entity).xpect_eq("div");
	}

	#[cfg(feature = "scene")]
	#[beet_core::test]
	fn parse_bsx() {
		let (world, entity) = parse(&MediaBytes::new_bsx("<div>hello</div>"));
		tag(&world, entity).xpect_eq("div");
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn parse_markdown() {
		let (world, entity) = parse(&MediaBytes::new_markdown("# Title"));
		let heading = world.get::<Children>(entity).unwrap()[0];
		tag(&world, heading).xpect_eq("h1");
	}

	/// Frontmatter is a property of the markdown format: with only the plain
	/// text fallback registered, a scan still reads a page's declarations.
	#[cfg(feature = "scene")]
	#[beet_core::test]
	fn text_fallback_reads_markdown_frontmatter() {
		let mut parsers = DocParsers::default()
			.with_metadata_type(TypeName::of_path::<DocMeta>());
		parsers.register(PlainTextParser::default());
		parsers
			.root_components(
				&MediaBytes::new_markdown(
					"+++\ntitle = \"Post\"\n[RouteMeta]\norder = 2\n+++\n\n# Post",
				),
				None,
			)
			.unwrap()
			.keys()
			.map(|name| name.as_str())
			.collect::<Vec<_>>()
			// no registry, so each is held by the name it was declared by
			.xpect_eq(vec![
				TypeName::of_path::<DocMeta>().as_str(),
				"RouteMeta",
			]);
		// a plain text document leading with a fence declares nothing
		parsers
			.root_components(
				&MediaBytes::new_text("+++\ntitle = \"x\"\n+++"),
				None,
			)
			.unwrap()
			.is_empty()
			.xpect_true();
	}

	/// A root carries its document's metadata, whichever markup declares it:
	/// frontmatter and a root spread read alike off the body's first entity.
	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn metadata_is_the_roots_components() {
		let world = world_ext::ui_world();
		let parsers = DocParsers::of(&world);
		let registry = world.resource::<AppTypeRegistry>().read();
		let title = |bytes: MediaBytes| {
			BodyComponents::get::<DocMeta>(
				&parsers.root_components(&bytes, None).unwrap(),
				&registry,
			)
			.unwrap()
			.unwrap()
			.title
		};
		let expected =
			title(MediaBytes::new_markdown("---\ntitle: Post\n---\n# Hi"));
		expected.as_deref().xpect_eq(Some("Post"));
		title(MediaBytes::new_bsx(
			r#"<main {DocMeta{title:"Post"}}><h1>Hi</h1></main>"#,
		))
		.xpect_eq(expected);
	}

	/// Mdx is markdown whose text holds expressions, its embedded BSX one tree
	/// with the prose around it.
	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn mdx_embeds_bsx() {
		let (mut world, entity) = parse(&MediaBytes::new_str(
			MediaType::Mdx,
			"# Hi {@doc:name}\n\n<section bx:scope=\"user\" {Name(\"Card\")}><p>a</p></section>",
		));
		let children = world.get::<Children>(entity).unwrap().to_vec();
		tag(&world, children[0]).xpect_eq("h1");
		tag(&world, children[1]).xpect_eq("section");
		world
			.entity(children[1])
			.contains::<DocumentScope>()
			.xpect_true();
		world.query::<&FieldRef>().iter(&world).count().xpect_eq(1);
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn parse_yaml_frontmatter() {
		let (world, entity) = parse(&MediaBytes::new_markdown(
			"---\ntitle: Hello\ndescription: World\n---\n\n# Hello",
		));
		world
			.get::<DocMeta>(entity)
			.unwrap()
			.title
			.as_deref()
			.xpect_eq(Some("Hello"));
	}

	/// Parse markdown then render it back via [`HtmlRenderer`].
	#[cfg(feature = "markdown")]
	fn roundtrip(md: &str) -> String {
		let (mut world, entity) = parse(&MediaBytes::new_markdown(md));
		HtmlRenderer::new()
			.render(&mut RenderContext::new(
				&mut world,
				entity,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string()
	}

	/// An element one HTML block opens and a later one closes holds the
	/// markdown between them, ie a disclosure's body.
	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn html_blocks_span_markdown() {
		roundtrip(
			"<details><summary>Open me</summary>\n\nHidden **body**.\n\n</details>\n\nAfter",
		)
		.xpect_contains("<details><summary>Open me</summary><p>Hidden <strong>body</strong>.</p></details><p>After</p>")
		.xnot()
		.xpect_contains("html_block");
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_paragraph() {
		roundtrip("Hello world").xpect_eq("<p>Hello world</p>".to_string());
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_heading() {
		roundtrip("# Title").xpect_eq("<h1>Title</h1>".to_string());
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_emphasis() {
		roundtrip("*hello*").xpect_eq("<p><em>hello</em></p>".to_string());
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_strong() {
		roundtrip("**hello**")
			.xpect_eq("<p><strong>hello</strong></p>".to_string());
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_link() {
		let html = roundtrip("[click](https://example.com)");
		html.xpect_contains("<a")
			.xpect_contains("href=\"https://example.com\"")
			.xpect_contains("click")
			.xpect_contains("</a>");
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_unordered_list() {
		let html = roundtrip("- a\n- b");
		html.xpect_contains("<ul>")
			.xpect_contains("<li>")
			.xpect_contains("</li>")
			.xpect_contains("</ul>");
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_code_block() {
		let html = roundtrip("```rust\nfn main() {}\n```");
		html.xpect_contains("<pre>")
			.xpect_contains("<code")
			.xpect_contains("</code>")
			.xpect_contains("</pre>");
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_blockquote() {
		let html = roundtrip("> quoted text");
		html.xpect_contains("<blockquote>")
			.xpect_contains("quoted text")
			.xpect_contains("</blockquote>");
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_thematic_break() { roundtrip("---").xpect_contains("<hr />"); }

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_image() {
		let html = roundtrip("![alt](image.png)");
		html.xpect_contains("<img")
			.xpect_contains("src=\"image.png\"");
	}

	#[cfg(feature = "markdown")]
	#[beet_core::test]
	fn roundtrip_inline_code() {
		roundtrip("use `foo()` here").xpect_contains("<code>foo()</code>");
	}
}
