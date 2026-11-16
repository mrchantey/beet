//! The registry of document formats a world reads: each [`DocParser`] names the
//! media types it parses and installs itself into [`DocParsers`], the one path
//! every parse takes, the parsing counterpart of [`DocRenderers`].
//!
//! [`ParsePlugin`] installs the built-in parsers: plain text, and with their
//! features BSX, HTML and XML (`bsx`), markdown (`markdown_parser`) and the
//! Office formats (`ooxml`, through [`OoxmlPlugin`]).

use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;

/// Parses a document into a tree: reads `cx.bytes`, one of the media types it
/// names, into `cx.entity` synchronously, keeping every ECS mutation in one
/// place. Registered in [`DocParsers`] by installing it,
/// `app.install(MarkdownParser::new())`. Its [`Installer`] impl, a supertrait
/// so no parser skips it, forwards to [`DocParsers::register_on`].
///
/// The registry clones the registered instance for every parse, so its
/// configuration is set once at registration.
pub trait DocParser: 'static + Send + Sync + Clone + Installer {
	/// The media types this parser reads, preferred first.
	fn media_types(&self) -> Vec<MediaType>;

	/// Parse the bytes in `cx` into `cx.entity`, returning
	/// [`ParseError::UnsupportedType`] for a media type it does not read.
	fn parse(&mut self, cx: ParseContext<'_, '_>) -> Result<(), ParseError>;

	/// The components `bytes` declares at its document root, ie markdown's
	/// frontmatter or markup's root spreads, its unsectioned keys naming
	/// `frontmatter_type`. A format with no declaration surface declares
	/// nothing.
	#[cfg(feature = "bsx")]
	fn declarations(
		&self,
		_bytes: &MediaBytes,
		_frontmatter_type: &str,
	) -> Result<RootDeclarations> {
		RootDeclarations::default().xok()
	}
}

/// Every [`DocParser`] this world reads, the one place a format's parser is
/// chosen and the one path a parse takes.
///
/// A parser is installed by value, `app.install(MarkdownParser::new())`, the
/// built-in parsers by [`ParsePlugin`] through the same call a downstream
/// parser uses. For an overlapping media type the parser registered last wins,
/// so a downstream crate replaces a built-in by registering its own. A text
/// type with no parser of its own falls back to the [`MediaType::Text`]
/// parser, since any text reads as a single text node.
///
/// Every parse ends in one [`PostParseTree`] run, whatever the format: the
/// schedule [`ParsePlugin`] fills, where the document's tables are numbered
/// ([`NumberTables`]) and whatever else extends a parsed tree runs, ie styles
/// or a deck's triage.
#[derive(Default, Clone, Resource)]
pub struct DocParsers {
	parsers: Vec<Arc<dyn ErasedDocParser>>,
}

impl DocParsers {
	/// Register `parser`, answering its media types ahead of any parser
	/// registered before it.
	pub fn register(&mut self, parser: impl DocParser) -> &mut Self {
		self.parsers.push(Arc::new(parser));
		self
	}

	/// [`register`](Self::register) `parser` in `app`'s registry, the body of
	/// every [`DocParser`]'s [`Installer`] impl.
	pub fn register_on(app: &mut App, parser: impl DocParser) {
		app.world_mut()
			.get_resource_or_init::<Self>()
			.register(parser);
	}

	/// Register the built-in parsers: plain text, and with their features
	/// BSX, HTML and XML (`bsx`) and markdown (`markdown_parser`), as
	/// [`ParsePlugin`] does, for a caller with no app, ie a codegen scan.
	pub fn register_builtins(&mut self) -> &mut Self {
		self.register(PlainTextParser::default());
		#[cfg(feature = "bsx")]
		self.register(BsxParser);
		#[cfg(feature = "markdown_parser")]
		self.register(MarkdownParser::new());
		self
	}

	/// Every media type a registered parser reads, in registration order.
	pub fn available(&self) -> Vec<MediaType> {
		let mut available = Vec::new();
		for media_type in
			self.parsers.iter().flat_map(|parser| parser.media_types())
		{
			if !available.contains(&media_type) {
				available.push(media_type);
			}
		}
		available
	}

	/// Parse `cx.bytes` into `cx.entity` through the parser its media type
	/// selects, then extend the parsed tree in one [`PostParseTree`] run.
	pub fn parse(cx: ParseContext) -> Result<(), ParseError> {
		let ParseContext {
			entity,
			bytes,
			path,
		} = cx;
		let parser = entity
			.world()
			.get_resource::<Self>()
			.ok_or_else(Self::missing)?
			.choose(bytes.media_type())?;
		parser.parse(ParseContext {
			entity: &mut *entity,
			bytes,
			path,
		})?;
		// the parsed tree is extended once, whatever its format: its tables
		// numbered, then whatever else is registered, ie styles or a deck's
		// triage
		if !entity.contains::<NumberTables>() {
			entity.insert(NumberTables::default());
		}
		entity.world_scope(|world| {
			let _ = world.try_run_schedule(PostParseTree);
		});
		Ok(())
	}

	/// The components `bytes` declares at its document root, through the
	/// parser its media type selects (see [`DocParser::declarations`]).
	///
	/// Deliberately not part of [`parse`](Self::parse): this reads the root
	/// only, building nothing, so a route scan can know a page's title, order
	/// and slug without spawning its content. A media type no parser reads
	/// declares nothing; a source that does not parse is an error, since the
	/// same bytes are about to be parsed for real.
	#[cfg(feature = "bsx")]
	pub fn declarations(
		&self,
		bytes: &MediaBytes,
		frontmatter_type: &str,
	) -> Result<RootDeclarations> {
		match self.choose(bytes.media_type()) {
			Ok(parser) => parser.declarations(bytes, frontmatter_type),
			Err(_) => RootDeclarations::default().xok(),
		}
	}

	/// The [`DocParsers`] of `world`, cloned out for an async scan.
	pub fn of(world: &World) -> Result<Self> {
		world
			.get_resource::<Self>()
			.cloned()
			.ok_or_else(|| Self::missing().into())
	}

	fn missing() -> ParseError {
		ParseError::Other(bevyhow!(
			"no `DocParsers` in this world: add the `ParsePlugin`"
		))
	}

	/// The parser reading `media_type`, the last registered winning, falling
	/// back to the plain text parser for any other text type.
	fn choose(
		&self,
		media_type: &MediaType,
	) -> Result<Arc<dyn ErasedDocParser>, ParseError> {
		let find = |media_type: &MediaType| {
			self.parsers
				.iter()
				.rev()
				.find(|parser| parser.media_types().contains(media_type))
				.cloned()
		};
		find(media_type)
			.or_else(|| {
				media_type
					.is_text()
					.then(|| find(&MediaType::Text))
					.flatten()
			})
			.ok_or_else(|| ParseError::UnsupportedType {
				unsupported: media_type.clone(),
				supported: self.available(),
			})
	}
}

/// The object-safe face of a [`DocParser`], which a registry can hold.
trait ErasedDocParser: 'static + Send + Sync {
	fn media_types(&self) -> Vec<MediaType>;
	fn parse(&self, cx: ParseContext) -> Result<(), ParseError>;
	#[cfg(feature = "bsx")]
	fn declarations(
		&self,
		bytes: &MediaBytes,
		frontmatter_type: &str,
	) -> Result<RootDeclarations>;
}

impl<T: DocParser> ErasedDocParser for T {
	fn media_types(&self) -> Vec<MediaType> { DocParser::media_types(self) }

	fn parse(&self, cx: ParseContext) -> Result<(), ParseError> {
		DocParser::parse(&mut self.clone(), cx)
	}

	#[cfg(feature = "bsx")]
	fn declarations(
		&self,
		bytes: &MediaBytes,
		frontmatter_type: &str,
	) -> Result<RootDeclarations> {
		DocParser::declarations(self, bytes, frontmatter_type)
	}
}






#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Parse `bytes` into a fresh entity of a world with the built-in parsers.
	fn parse(bytes: &MediaBytes) -> Result<(World, Entity), ParseError> {
		let mut world = world_ext::ui_world();
		let entity = world.spawn_empty().id();
		DocParsers::parse(ParseContext::new(
			&mut world.entity_mut(entity),
			bytes,
		))?;
		Ok((world, entity))
	}

	/// The text of `entity`'s first child.
	fn first_text(world: &mut World, entity: Entity) -> Value {
		world
			.entity_mut(entity)
			.child(0)
			.unwrap()
			.get::<Value>()
			.cloned()
			.unwrap()
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

	#[beet_core::test]
	fn parse_plain_text() {
		let (mut world, entity) =
			parse(&MediaBytes::new_text("hello")).unwrap();
		first_text(&mut world, entity).xpect_eq(Value::Str("hello".into()));
	}

	/// `MediaType::Html` reads through the BSX parser, HTML being the
	/// features-off subset.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn parse_html() {
		let (mut world, entity) =
			parse(&MediaBytes::new_html("<div>hello</div>")).unwrap();
		first_tag(&mut world, entity).xpect_eq("div".to_string());
	}

	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn parse_bsx() {
		let (mut world, entity) =
			parse(&MediaBytes::new_bsx("<div>hello</div>")).unwrap();
		first_tag(&mut world, entity).xpect_eq("div".to_string());
	}

	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn parse_markdown() {
		let (mut world, entity) =
			parse(&MediaBytes::new_markdown("# Title")).unwrap();
		first_tag(&mut world, entity).xpect_eq("h1".to_string());
	}

	/// Frontmatter is a property of the markdown format: with only the plain
	/// text fallback registered, a scan still reads a page's declarations.
	#[cfg(feature = "bsx")]
	#[beet_core::test]
	fn text_fallback_reads_markdown_frontmatter() {
		let mut parsers = DocParsers::default();
		parsers.register(PlainTextParser::default());
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

	/// CSS has no parser of its own but is a text type, so it reads as plain
	/// text.
	#[beet_core::test]
	fn text_falls_back_to_plain_text() {
		let (mut world, entity) =
			parse(&MediaBytes::new_css("body { color: red; }")).unwrap();
		first_text(&mut world, entity)
			.xpect_eq(Value::Str("body { color: red; }".into()));
	}

	#[beet_core::test]
	fn binary_type_errors() {
		parse(&MediaBytes::new(MediaType::Png, b"\x89PNG".as_slice()))
			.unwrap_err();
	}
}
