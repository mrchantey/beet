//! The markup formats, and scheduling for the post-parse pipeline.
//!
//! The format registry is `beet_core`'s [`DocParsers`], where a parser returns
//! a scene body and the base formats (plain text, the structured ones) are
//! always present; [`ParsePlugin`] adds the markup formats this crate reads,
//! BSX, HTML and XML (`bsx`) and markdown (`markdown_parser`). The Office
//! formats (`ooxml`) are `OoxmlPlugin`'s, which a router installs: their
//! writers resolve looks through the style plugin, itself built over this one.
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
		parsers
			.register(
				PlainTextParser::default()
					.with_declarations(Frontmatter::root_declarations),
			)
			.register(BsxParser);
		#[cfg(feature = "markdown_parser")]
		parsers.register(MarkdownParser::new());
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

	/// The tag of `entity`'s first child.
	#[cfg(feature = "bsx")]
	fn first_tag(world: &mut World, entity: Entity) -> String {
		world
			.entity_mut(entity)
			.child(0)
			.unwrap()
			.get::<Element>()
			.unwrap()
			.tag()
			.to_string()
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
		let (mut world, entity) =
			parse(&MediaBytes::new_html("<div>hello</div>"));
		first_tag(&mut world, entity).xpect_eq("div".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn parse_bsx() {
		let (mut world, entity) =
			parse(&MediaBytes::new_bsx("<div>hello</div>"));
		first_tag(&mut world, entity).xpect_eq("div".to_string());
	}

	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn parse_markdown() {
		let (mut world, entity) = parse(&MediaBytes::new_markdown("# Title"));
		first_tag(&mut world, entity).xpect_eq("h1".to_string());
	}

	/// A markup parser builds into a world, so it has no body to return.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn markup_has_no_body_yet() {
		ParsePlugin::parsers()
			.body(&MediaBytes::new_bsx("<div/>"))
			.unwrap_err()
			.to_string()
			.xpect_contains("parses into a world");
	}

	/// Frontmatter is a property of the markdown format: with only the plain
	/// text fallback registered, a scan still reads a page's declarations.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn text_fallback_reads_markdown_frontmatter() {
		let mut parsers = DocParsers::default();
		parsers.register(
			PlainTextParser::default()
				.with_declarations(Frontmatter::root_declarations),
		);
		let declarations = parsers
			.declarations(
				&MediaBytes::new_markdown(
					"+++\ntitle = \"Post\"\n[RouteMeta]\norder = 2\n+++\n\n# Post",
				),
				"DocMeta",
			)
			.unwrap();
		declarations
			.0
			.iter()
			.map(|literal| literal.name.as_str())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["DocMeta", "RouteMeta"]);
		// a plain text document leading with a fence declares nothing
		parsers
			.declarations(
				&MediaBytes::new_text("+++\ntitle = \"x\"\n+++"),
				"DocMeta",
			)
			.unwrap()
			.is_empty()
			.xpect_true();
	}
}
