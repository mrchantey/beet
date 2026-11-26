//! The format registry: each [`DocParser`] names the media types it reads and
//! turns their bytes into a [`ParsedDocument`], and [`DocParsers`] is the one place
//! a format's parser is chosen, the parsing counterpart of `beet_ui`'s
//! `DocRenderers`.
//!
//! What a parser reads is a scene document: a document compiled into a scene,
//! strictly upstream of it and never written back from a running world. The
//! distinction from a state document, the [`Document`] a widget binds to, is
//! stated once, in the `document` module docs.
//!
//! A parser touches no world: bytes in, scene body out, with where each entity
//! of it was written beside it ([`SourceMap`]), and everything after
//! the body is one pipeline that knows nothing of the format it came from.
//!
//! [`ScenePlugin`] registers the base formats, and `beet_ui` adds the office
//! formats and mermaid. A file whose media type has no parser is loaded as its
//! blob.

use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use core::any::Any;

/// Reads the bytes of the media types it names into a [`ParsedDocument`],
/// touching no world. Registered in [`DocParsers`] by installing it,
/// `app.install(MyParser)`; its [`Installer`] impl, a
/// supertrait so no parser skips it, forwards to [`DocParsers::register_on`].
///
/// The registry clones the registered instance for every parse, so its
/// configuration is set once at registration. A parser with an intermediate
/// of its own (a markup syntax tree) exposes it as its own method, so a tool
/// may stop there; the trait knows only bytes in, scene body out.
///
/// A format with positions fills the document's [`SourceMap`], and an error it
/// raises somewhere in particular is a [`SourceError`].
pub trait DocParser: 'static + Send + Sync + Clone + Installer {
	/// The media types this parser reads, preferred first.
	fn media_types(&self) -> Vec<MediaType>;

	/// What a markdown fence naming this format becomes when it does not say:
	/// a diagram is drawn, a sample of markup stays code.
	fn fence_render(&self) -> FenceRender { FenceRender::Code }

	/// Configure this parse with what a fence says beyond its format and
	/// `render`, ie `form=text` of ```` ```mermaid form=text ````, on the clone
	/// that parses it. A format taking no params refuses any.
	fn configure(&mut self, params: &FenceParams) -> Result {
		params.assert_empty()
	}

	/// Bytes of one of those media types to a scene body and where it was
	/// written, touching no world.
	/// `parsers` is the registry this was chosen from, what a format that
	/// names things lowers against: its [types](DocParsers::types) and the
	/// formats it embeds ([`get`](DocParsers::get)).
	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError>;
}

/// Every [`DocParser`] this world reads.
///
/// A parser is installed by value, `app.install(MyParser)`. For an
/// overlapping media type the parser registered last wins, so a downstream
/// crate replaces a base format by registering its own, a
/// [base parser](Self::register_base) ranking behind every other whenever it
/// arrives. A text type with no parser of its own falls back to the
/// [`MediaType::Text`] parser, since any text reads as a single text entity.
///
/// It carries what every lowering reads with no world: the type registry names
/// resolve against and the component a document's unsectioned metadata
/// declares. What a format reads of another it embeds, ie markdown of BSX's
/// directives, is that format's registered parser ([`get`](Self::get)).
#[derive(Clone, Resource)]
pub struct DocParsers {
	parsers: Vec<Arc<dyn ErasedDocParser>>,
	types: AppTypeRegistry,
	metadata_type: ReflectName,
}

/// No parsers and no types, so nothing reads and every name is unregistered
/// until it reaches a world.
impl Default for DocParsers {
	fn default() -> Self {
		Self {
			parsers: default(),
			types: default(),
			metadata_type: ReflectName::new(Self::METADATA_TYPE),
		}
	}
}

impl DocParsers {
	/// What unsectioned metadata declares until a plugin names its own
	/// component: a name nothing registers, so it is held with its values.
	pub const METADATA_TYPE: &'static str = "Metadata";

	/// Register `parser`, answering its media types ahead of any parser
	/// registered before it.
	pub fn register(&mut self, parser: impl DocParser) -> &mut Self {
		self.parsers.push(Arc::new(parser));
		self
	}

	/// Register `parser` behind every other, so one registered before it still
	/// answers for their shared media types.
	pub fn register_base(&mut self, parser: impl DocParser) -> &mut Self {
		self.parsers.insert(0, Arc::new(parser));
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
	/// markdown page's frontmatter keys:
	/// [`METADATA_TYPE`](Self::METADATA_TYPE) until a plugin names one.
	pub fn metadata_type(&self) -> &ReflectName { &self.metadata_type }

	/// Declare unsectioned metadata as the component `metadata_type` names.
	pub fn with_metadata_type(
		mut self,
		metadata_type: impl Into<ReflectName>,
	) -> Self {
		self.set_metadata_type(metadata_type);
		self
	}

	/// Declare unsectioned metadata as the component `metadata_type` names.
	pub fn set_metadata_type(
		&mut self,
		metadata_type: impl Into<ReflectName>,
	) -> &mut Self {
		self.metadata_type = metadata_type.into();
		self
	}

	/// The parser of type `T` registered last, ie the BSX parser whose
	/// directives markdown's embedded markup reads.
	pub fn get<T: DocParser>(&self) -> Option<&T> {
		self.parsers
			.iter()
			.rev()
			.find_map(|parser| parser.as_any().downcast_ref::<T>())
	}

	/// Edit the parser of type `T` registered last, registering its default as
	/// a [base](Self::register_base) where there is none.
	pub fn edit<T: DocParser + Default>(
		&mut self,
		edit: impl FnOnce(&mut T),
	) -> &mut Self {
		let index = self
			.parsers
			.iter()
			.rposition(|parser| parser.as_any().is::<T>());
		let mut parser = index
			.and_then(|index| {
				self.parsers[index].as_any().downcast_ref::<T>().cloned()
			})
			.unwrap_or_default();
		edit(&mut parser);
		match index {
			Some(index) => self.parsers[index] = Arc::new(parser),
			None => self.parsers.insert(0, Arc::new(parser)),
		}
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

	/// Whether a parser reads `media_type` itself, a text type's fallback to
	/// plain text aside.
	pub fn reads(&self, media_type: &MediaType) -> bool {
		self.parsers
			.iter()
			.any(|parser| parser.media_types().contains(media_type))
	}

	/// What a fence naming `media_type` becomes when it does not say, `None`
	/// when no parser reads the type ([`DocParser::fence_render`]).
	pub fn fence_render(&self, media_type: &MediaType) -> Option<FenceRender> {
		self.reads(media_type)
			.then(|| self.choose(media_type).ok())
			.flatten()
			.map(|parser| parser.fence_render())
	}

	/// `bytes` through the parser their media type selects, an error's place
	/// an offset in them: what a document embedding another reads it by.
	pub fn parse(
		&self,
		bytes: &MediaBytes,
	) -> Result<ParsedDocument, ParseError> {
		self.choose(bytes.media_type())?.parse(bytes, self, None)
	}

	/// `bytes` as a fence holds them, the parser their media type selects
	/// [configured](DocParser::configure) with `params`.
	pub fn parse_fence(
		&self,
		bytes: &MediaBytes,
		params: &FenceParams,
	) -> Result<ParsedDocument, ParseError> {
		self.choose(bytes.media_type())?
			.parse(bytes, self, Some(params))
	}

	/// The document `bytes` are, the file at `path` of its package where it
	/// has one, through the parser their media type selects.
	///
	/// ## Errors
	/// Errors naming `path:line:col` when the document does not read, as much
	/// of it as is known.
	pub fn read(
		&self,
		bytes: &MediaBytes,
		path: Option<&RelPath>,
	) -> Result<DocumentSource, ParseError> {
		match self.parse(bytes) {
			Ok(document) => {
				DocumentSource::new(path.cloned(), bytes.clone(), document)
					.xok()
			}
			Err(error) => Err(error.locate(bytes, path)),
		}
	}

	/// `bytes` as a scene body, through the parser their media type selects.
	pub fn body(&self, bytes: &MediaBytes) -> Result<SceneBody, ParseError> {
		match self.parse(bytes) {
			Ok(document) => document.body.xok(),
			Err(error) => Err(error.locate(bytes, None)),
		}
	}

	/// The components `bytes`, the document at `path` of its package where it
	/// has one, declares at its root, ie a markdown
	/// page's frontmatter or a BSX page's root spreads: its first entity's
	/// ([`EntityBody::declarations`]).
	///
	/// Nothing is spawned, so a route scan knows a page's title, order and
	/// slug without building its content. A media type no parser reads
	/// declares nothing; a source that does not parse is an error, since the
	/// same bytes are about to be parsed for real.
	pub fn root_components(
		&self,
		bytes: &MediaBytes,
		path: Option<&RelPath>,
	) -> Result<EntityBody> {
		match self.choose(bytes.media_type()) {
			Ok(parser) => parser
				.parse(bytes, self, None)
				.map_err(|error| error.locate(bytes, path))?
				.body
				.entities
				.into_iter()
				.next()
				.map(|(_, root)| root.declarations())
				.unwrap_or_default()
				.xok(),
			Err(_) => EntityBody::default().xok(),
		}
	}

	/// The [`DocParsers`] of `world`, cloned out for an async scan, lowering
	/// against its types.
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

/// The object-safe face of a [`DocParser`], which a registry can hold.
trait ErasedDocParser: 'static + Send + Sync {
	fn as_any(&self) -> &dyn Any;
	fn media_types(&self) -> Vec<MediaType>;
	fn fence_render(&self) -> FenceRender;
	/// Parse with a clone of this parser, configured by `params` where a
	/// fence gives some.
	fn parse(
		&self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
		params: Option<&FenceParams>,
	) -> Result<ParsedDocument, ParseError>;
}

impl<T: DocParser> ErasedDocParser for T {
	fn as_any(&self) -> &dyn Any { self }

	fn media_types(&self) -> Vec<MediaType> { DocParser::media_types(self) }

	fn fence_render(&self) -> FenceRender { DocParser::fence_render(self) }

	fn parse(
		&self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
		params: Option<&FenceParams>,
	) -> Result<ParsedDocument, ParseError> {
		let mut parser = self.clone();
		if let Some(params) = params {
			parser.configure(params)?;
		}
		DocParser::parse(&mut parser, bytes, parsers)
	}
}
