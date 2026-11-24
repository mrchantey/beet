//! The markup formats, and scheduling for the post-parse pipeline.
//!
//! The format registry is `beet_core`'s [`DocParsers`], where a parser returns
//! a scene body and the base formats (plain text, the structured ones, BSX,
//! HTML and XML) are always present; [`ParsePlugin`] adds the formats this
//! crate reads, markdown and mdx (`markdown_parser`). The Office formats
//! (`ooxml`) are `OoxmlPlugin`'s, which a router installs: their writers
//! resolve looks through the style plugin, itself built over this one.
//!
//! [`PostParseTree`] is the single schedule where a parsed entity tree is
//! styled, decorated, laid out, and painted. Every render path runs it
//! on demand: [`DocParsers::parse`] and the charcell renderers call
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

/// Registers the markup [`DocParsers`] and the [`PostParseTree`] schedule
/// for on-demand runs.
///
/// Does not add the schedule to the main schedule order, see the module docs.
/// Realtime apps add [`RealtimeParsePlugin`] for per-frame repaint.
#[derive(Default)]
pub struct ParsePlugin;

impl ParsePlugin {
	/// The parsers this plugin registers beside the base ones, for a caller
	/// with no app, ie a codegen scan.
	pub fn parsers() -> DocParsers {
		let mut parsers = DocParsers::default();
		Self::register_markup(&mut parsers);
		parsers
	}

	/// Register the markup formats this build reads. The plain text fallback
	/// reads markdown frontmatter, so a route scan reads a page's metadata
	/// with no markdown parser.
	fn register_markup(parsers: &mut DocParsers) {
		#[cfg(feature = "bsx")]
		parsers.register(
			PlainTextParser::default()
				.with_root_components(Frontmatter::root_components),
		);
		#[cfg(feature = "markdown_parser")]
		parsers
			.register(MarkdownParser::new())
			.register(MarkdownParser::mdx());
		#[cfg(not(any(feature = "bsx", feature = "markdown_parser")))]
		let _ = parsers;
	}
}

impl Plugin for ParsePlugin {
	fn build(&self, app: &mut App) {
		// ensure the schedule exists for on-demand `run_schedule` callers, without
		// adding it to the per-frame main loop (which would repaint the whole world
		// every tick).
		app.init_schedule(PostParseTree);
		// the markup formats beside the base ones; `OoxmlPlugin`, which
		// installs the style plugin over this, adds the Office formats
		Self::register_markup(
			&mut app.world_mut().get_resource_or_init::<DocParsers>(),
		);
		// the default BSX event vocabulary, so every `bx:onclick="..."` runs
		// its script on a click.
		#[cfg(feature = "bsx")]
		app.install_type::<crate::prelude::BsxDefaultsPlugin>();
		// a document's root metadata and its route's, which frontmatter
		// declares and writes
		app.register_type::<DocMeta>()
			.register_type::<RouteMeta>()
			.register_type::<InlineBlob>();
		// the limits and vocabulary `beet check` holds every page's metadata to
		app.register_type::<DocMetaRules>()
			.register_type::<TextLimit>();
		#[cfg(feature = "bsx")]
		app.register_type::<FrontmatterType>();
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
		app.install_type::<ParsePlugin>()
			.install(InsertSchedule::after(Update, PostParseTree));
	}
}

#[cfg(test)]
mod test {
	use super::*;
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
		DocParsers::parse(ParseContext::new(
			&mut world.entity_mut(entity),
			bytes,
		))
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
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn parse_html() {
		let (world, entity) = parse(&MediaBytes::new_html("<div>hello</div>"));
		tag(&world, entity).xpect_eq("div");
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn parse_bsx() {
		let (world, entity) = parse(&MediaBytes::new_bsx("<div>hello</div>"));
		tag(&world, entity).xpect_eq("div");
	}

	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn parse_markdown() {
		let (world, entity) = parse(&MediaBytes::new_markdown("# Title"));
		let heading = world.get::<Children>(entity).unwrap()[0];
		tag(&world, heading).xpect_eq("h1");
	}

	/// Frontmatter is a property of the markdown format: with only the plain
	/// text fallback registered, a scan still reads a page's declarations.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn text_fallback_reads_markdown_frontmatter() {
		let mut parsers = DocParsers::default();
		parsers.register(
			PlainTextParser::default()
				.with_root_components(Frontmatter::root_components),
		);
		parsers
			.root_components(&MediaBytes::new_markdown(
				"+++\ntitle = \"Post\"\n[RouteMeta]\norder = 2\n+++\n\n# Post",
			))
			.unwrap()
			.keys()
			.map(|name| name.as_str())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["DocMeta", "RouteMeta"]);
		// a plain text document leading with a fence declares nothing
		parsers
			.root_components(&MediaBytes::new_text("+++\ntitle = \"x\"\n+++"))
			.unwrap()
			.is_empty()
			.xpect_true();
	}

	/// A root carries its document's metadata, whichever markup declares it:
	/// frontmatter and a root spread read alike off the body's first entity.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn metadata_is_the_roots_components() {
		let world = world_ext::ui_world();
		let parsers = DocParsers::of(&world);
		let registry = world.resource::<AppTypeRegistry>().read();
		let title = |bytes: MediaBytes| {
			parsers
				.root_components(&bytes)
				.unwrap()
				.get_reflect::<DocMeta>(&registry)
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
	#[cfg(feature = "markdown_parser")]
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
}
