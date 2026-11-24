//! The format registry: each [`DocParser`] names the media types it reads and
//! turns their bytes into a [`SceneBody`], and [`DocParsers`] is the one place
//! a format's parser is chosen and the one path a parse into a world takes,
//! the parsing counterpart of `beet_ui`'s `DocRenderers`.
//!
//! What a parser reads is a scene document: a document compiled into a scene,
//! strictly upstream of it and never written back from a running world. The
//! distinction from a state document, the [`Document`] a widget binds to, is
//! stated once, in the `document` module docs.
//!
//! A parser touches no world: bytes in, scene body out, and everything after
//! the body is one pipeline that knows nothing of the format it came from.
//! [`DocParsers::parse`] is that pipeline: choose the parser, parse, spawn the
//! body onto the entity, then one [`PostParseTree`] run.
//!
//! The base formats are in every registry, [`DocParsers::default`], so a
//! world with none installed still reads them:
//!
//! ```text
//! application/json,                       a scene body when the root object
//! application/postcard, application/toml  carries `$type: org.beet.scene#body`,
//!                                         else one entity holding the `Value`
//! text/javascript                         one entity carrying `RunScript`, the
//!                                         source as its `Value`
//! text/plain, any other text type         one entity holding the text
//! text/bsx, text/html, xml                a scene body (`bsx` feature)
//! ```
//!
//! `beet_ui` adds markdown, mdx and the office formats. A file whose media
//! type has no parser is loaded as its blob.

use crate::prelude::*;
use alloc::sync::Arc;
use bevy::ecs::schedule::ScheduleLabel;

/// Reads the bytes of the media types it names into a [`SceneBody`],
/// touching no world. Registered in [`DocParsers`] by installing it,
/// `app.install(PlainTextParser::default())`; its [`Installer`] impl, a
/// supertrait so no parser skips it, forwards to [`DocParsers::register_on`].
///
/// The registry clones the registered instance for every parse, so its
/// configuration is set once at registration. A parser with an intermediate
/// of its own (a markup syntax tree) exposes it as its own method, so a tool
/// may stop there; the trait knows only bytes in, scene body out.
pub trait DocParser: 'static + Send + Sync + Clone + Installer {
	/// The media types this parser reads, preferred first.
	fn media_types(&self) -> Vec<MediaType>;

	/// Bytes of one of those media types to a scene body, touching no world.
	/// `parsers` is the registry this was chosen from, what a format that
	/// names things lowers against: its [types](DocParsers::types) and its
	/// [directives](DocParsers::directives).
	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<SceneBody, ParseError>;
}

/// Every [`DocParser`] this world reads.
///
/// A parser is installed by value, `app.install(PlainTextParser::default())`,
/// and the base parsers (plain text, javascript and the structured formats
/// this build reads) are in every registry from the start. For an overlapping media type the parser registered last
/// wins, so a downstream crate replaces a built-in by registering its own. A
/// text type with no parser of its own falls back to the [`MediaType::Text`]
/// parser, since any text reads as a single text entity.
///
/// Every parse into a world ends in one [`PostParseTree`] run, whatever the
/// format, with the document's root marked [`ParsedRoot`] for its duration.
///
/// It carries what every markup lowering reads with no world: the
/// [`BsxDirectives`], the type registry names resolve against, and the
/// component a document's unsectioned metadata declares.
#[derive(Clone, Resource)]
pub struct DocParsers {
	parsers: Vec<Arc<dyn ErasedDocParser>>,
	types: AppTypeRegistry,
	metadata_type: SmolStr,
	#[cfg(feature = "bsx")]
	directives: BsxDirectives,
}

/// The base parsers: plain text, javascript, the structured formats and BSX
/// with HTML and XML, the standard directives, and no types, so every name a
/// document writes is unregistered until it reaches a world.
impl Default for DocParsers {
	fn default() -> Self {
		let mut parsers = Self {
			parsers: Vec::new(),
			types: default(),
			metadata_type: "DocMeta".into(),
			#[cfg(feature = "bsx")]
			directives: default(),
		};
		parsers
			.register(PlainTextParser::default())
			.register(ScriptParser)
			.register(StructuredParser);
		#[cfg(feature = "bsx")]
		parsers.register(BsxParser);
		parsers
	}
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

	/// The types a markup lowering resolves names against: a world's own where
	/// this came from one ([`of`](Self::of)), else none.
	pub fn types(&self) -> &AppTypeRegistry { &self.types }

	/// The component a document's unsectioned metadata declares, ie a
	/// markdown page's frontmatter keys, by short name.
	pub fn metadata_type(&self) -> &str { &self.metadata_type }

	/// Declare unsectioned metadata as the component `metadata_type` names.
	pub fn with_metadata_type(
		mut self,
		metadata_type: impl Into<SmolStr>,
	) -> Self {
		self.metadata_type = metadata_type.into();
		self
	}

	/// The directives every BSX lowering rewrites.
	#[cfg(feature = "bsx")]
	pub fn directives(&self) -> &BsxDirectives { &self.directives }

	/// The directives, to register one ([`BsxDirectives::register_on`]).
	#[cfg(feature = "bsx")]
	pub fn directives_mut(&mut self) -> &mut BsxDirectives {
		&mut self.directives
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

	/// `bytes` as a scene body, through the parser its media type selects.
	pub fn body(&self, bytes: &MediaBytes) -> Result<SceneBody, ParseError> {
		self.choose(bytes.media_type())?.parse(bytes, self)
	}

	/// Parse `cx.bytes` onto `cx.entity`, its root the entity itself, through
	/// the parser its media type selects, then extend the parsed tree in one
	/// [`PostParseTree`] run.
	pub fn parse(cx: ParseContext) -> Result<(), ParseError> {
		let ParseContext {
			entity,
			bytes,
			path,
			metadata_type,
		} = cx;
		let mut parsers = Self::of(entity.world());
		if let Some(metadata_type) = metadata_type {
			parsers = parsers.with_metadata_type(metadata_type);
		}
		let body = parsers.body(bytes)?;
		let template = match path {
			Some(path) => body.reflect_at(entity.world(), &path),
			None => body.reflect(entity.world()),
		}?;
		Self::build(entity, template)?;
		Ok(())
	}

	/// Build `template` onto `entity`, its root the entity itself, then extend
	/// the tree in one [`PostParseTree`] run: what [`parse`](Self::parse) does
	/// with the body it read.
	pub fn build(
		entity: &mut EntityWorldMut,
		template: DynamicTemplate,
	) -> Result {
		let id = entity.id();
		entity.world_scope(|world| {
			TemplateLoader::new(world)
				.with_entity(id)
				.build_onto(template)
		})?;
		// the parsed tree is extended once, whatever its format: its tables
		// numbered, then whatever else is registered, ie styles or a deck's
		// triage
		entity.insert(ParsedRoot);
		entity.world_scope(|world| {
			let _ = world.try_run_schedule(PostParseTree);
		});
		entity.remove::<ParsedRoot>();
		Ok(())
	}

	/// The components `bytes` declares at its document root, ie a markdown
	/// page's frontmatter or a BSX page's root spreads: its first entity's
	/// ([`EntityBody::declarations`]).
	///
	/// Nothing is spawned, so a route scan knows a page's title, order and
	/// slug without building its content. A media type no parser reads
	/// declares nothing; a source that does not parse is an error, since the
	/// same bytes are about to be parsed for real.
	pub fn root_components(&self, bytes: &MediaBytes) -> Result<EntityBody> {
		match self.choose(bytes.media_type()) {
			Ok(parser) => parser
				.parse(bytes, self)?
				.entities
				.into_iter()
				.next()
				.map(|(_, root)| root.declarations())
				.unwrap_or_default()
				.xok(),
			Err(_) => EntityBody::default().xok(),
		}
	}

	/// The [`DocParsers`] of `world`, cloned out for an async scan, the base
	/// parsers where it has installed none, lowering against its types.
	pub fn of(world: &World) -> Self {
		let mut parsers =
			world.get_resource::<Self>().cloned().unwrap_or_default();
		if let Some(types) = world.get_resource::<AppTypeRegistry>() {
			parsers.types = types.clone();
		}
		parsers
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

/// The schedule run once a document has been parsed into a world, its root
/// marked [`ParsedRoot`] for the run (or, with `beet_ui`'s
/// `RealtimeParsePlugin`, after every [`Update`]).
///
/// Hosts table numbering, style resolution, syntax highlighting, charcell
/// decorations and the layout and paint pipeline, ordered by their system
/// sets.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct PostParseTree;

/// The root of a document [`DocParsers::parse`] has just parsed, present for
/// the [`PostParseTree`] run that follows, so a step extending every parsed
/// tree finds the roots that are new.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct ParsedRoot;

/// The object-safe face of a [`DocParser`], which a registry can hold.
trait ErasedDocParser: 'static + Send + Sync {
	fn media_types(&self) -> Vec<MediaType>;
	fn parse(
		&self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<SceneBody, ParseError>;
}

impl<T: DocParser> ErasedDocParser for T {
	fn media_types(&self) -> Vec<MediaType> { DocParser::media_types(self) }

	fn parse(
		&self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<SceneBody, ParseError> {
		DocParser::parse(&mut self.clone(), bytes, parsers)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A world reading the base formats.
	fn world() -> World { TemplatePlugin::world() }

	/// Parse `bytes` onto a fresh entity of `world`.
	fn parse(
		world: &mut World,
		bytes: &MediaBytes,
	) -> Result<Entity, ParseError> {
		let entity = world.spawn_empty().id();
		DocParsers::parse(ParseContext::new(
			&mut world.entity_mut(entity),
			bytes,
		))?;
		Ok(entity)
	}

	/// A body of one root holding `value`.
	fn holding(value: Value) -> SceneBody {
		SceneBody::root(EntityBody::default().with_typed(&value).unwrap())
	}

	#[crate::test]
	fn plain_text_is_one_entity_holding_the_text() {
		DocParsers::of(&world())
			.body(&MediaBytes::new_text("hello"))
			.unwrap()
			.xpect_eq(holding(Value::Str("hello".into())));
	}

	/// CSS has no parser of its own but is a text type, so it reads as plain
	/// text, landing on the entity it parses onto.
	#[crate::test]
	fn text_falls_back_to_plain_text() {
		let mut world = world();
		let entity =
			parse(&mut world, &MediaBytes::new_css("body { color: red; }"))
				.unwrap();
		world
			.get::<Value>(entity)
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("body { color: red; }".into()));
		// the marker lasts the post parse run alone
		world.get::<ParsedRoot>(entity).xpect_none();
	}

	#[crate::test]
	fn binary_type_errors() {
		parse(
			&mut world(),
			&MediaBytes::new(MediaType::Png, b"\x89PNG".as_slice()),
		)
		.unwrap_err();
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn json_without_a_type_is_one_value() {
		DocParsers::of(&world())
			.body(&MediaBytes::new_json(r#"{"todos":["milk"]}"#))
			.unwrap()
			.xpect_eq(holding(value!({ "todos": ["milk"] })));
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn json_naming_the_def_is_a_body() {
		let body = DocParsers::of(&world())
			.body(&MediaBytes::new_json(
				r#"{"$type":"org.beet.scene#body","entities":{"0":{"Health":3}}}"#,
			))
			.unwrap();
		body.entities[&EntityKey::ROOT]
			.get("Health")
			.unwrap()
			.clone()
			.xpect_eq(Value::Uint(3));
	}

	#[cfg(feature = "toml")]
	#[crate::test]
	fn toml_reads_both_ways() {
		let parsers = DocParsers::of(&world());
		parsers
			.body(&MediaBytes::new_str(MediaType::Toml, "title = \"Post\""))
			.unwrap()
			.xpect_eq(holding(value!({ "title": "Post" })));
		parsers
			.body(&MediaBytes::new_str(
				MediaType::Toml,
				"\"$type\" = \"org.beet.scene#body\"\n[entities.0]\nHealth = 3",
			))
			.unwrap()
			.entities
			.len()
			.xpect_eq(1);
	}

	/// Postcard names nothing of itself, so a body is told from a value by
	/// the `$type` it is always written with.
	#[cfg(feature = "postcard")]
	#[crate::test]
	fn postcard_reads_both_ways() {
		let parsers = DocParsers::of(&world());
		let body = holding(Value::Str("hi".into()));
		parsers
			.body(&body.to_bytes(MediaType::Postcard).unwrap())
			.unwrap()
			.xpect_eq(body);
		let value = value!({ "a": [1u64, 2u64] });
		parsers
			.body(&MediaBytes::serialize(MediaType::Postcard, &value).unwrap())
			.unwrap()
			.xpect_eq(holding(value));
	}

	#[crate::test]
	fn javascript_is_one_script() {
		DocParsers::of(&world())
			.body(&MediaBytes::new_str(
				MediaType::Javascript,
				"console.log(1)",
			))
			.unwrap()
			.xpect_eq(SceneBody::root(
				EntityBody::default()
					.with_typed(&RunScript)
					.unwrap()
					.with_typed(&Value::Str("console.log(1)".into()))
					.unwrap(),
			));
	}

	/// A parser registered last answers its media types ahead of a built-in.
	#[crate::test]
	fn a_downstream_parser_replaces_a_builtin() {
		#[derive(Clone)]
		struct Shout;
		impl DocParser for Shout {
			fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Text] }
			fn parse(
				&mut self,
				bytes: &MediaBytes,
				_parsers: &DocParsers,
			) -> Result<SceneBody, ParseError> {
				let text = bytes.as_utf8()?.to_uppercase();
				SceneBody::root(
					EntityBody::default().with_typed(&Value::new(text))?,
				)
				.xok()
			}
		}
		impl Installer for Shout {
			fn install(self, app: &mut App) {
				DocParsers::register_on(app, self);
			}
		}
		let mut app = App::new();
		app.add_plugins(TemplatePlugin).install(Shout);
		let entity =
			parse(app.world_mut(), &MediaBytes::new_text("hi")).unwrap();
		app.world()
			.get::<Value>(entity)
			.cloned()
			.unwrap()
			.xpect_eq(Value::Str("HI".into()));
	}
}
