//! [`BsxParser`], markup as a scene body.
use super::lower::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// Reads BSX, HTML and XML as a [`SceneBody`], each in the dialect its media
/// type names ([`BsxParseConfig::from_media_type`]): BSX the full grammar,
/// HTML the same with the BSX surface off, XML markup exactly as written.
///
/// The syntax tree between is [`BsxNode::parse_document`]'s, for a tool that
/// stops there.
#[derive(Debug, Clone, Default)]
pub struct BsxParser;

impl BsxParser {
	/// Lower a document's nodes to a body, its directives rewritten first. A
	/// document with one root element is that element, see the `lower` module
	/// docs.
	pub fn lower(
		nodes: Vec<BsxNode>,
		parsers: &DocParsers,
	) -> Result<ParsedDocument> {
		Self::lower_nodes(nodes, LowerRoot::Document, parsers)
	}

	/// Lower a document whose root is the document rather than an element of
	/// it, ie markdown: the root holds `root` and every node is its child.
	pub fn lower_container(
		nodes: Vec<BsxNode>,
		root: EntityBody,
		parsers: &DocParsers,
	) -> Result<ParsedDocument> {
		Self::lower_nodes(nodes, LowerRoot::Container(root), parsers)
	}

	/// Lower `declarations` to the components they name, as the root spreads
	/// of a document declaring them would: how a format with metadata of its
	/// own (frontmatter, a file's properties) declares its root's components.
	pub fn lower_declarations(
		declarations: impl IntoIterator<Item = NamedLiteral>,
		parsers: &DocParsers,
	) -> Result<EntityBody> {
		let items = declarations
			.into_iter()
			.map(SpreadItem::Named)
			.collect::<Vec<_>>();
		if items.is_empty() {
			return EntityBody::default().xok();
		}
		let mut root = BsxElement::new("Fragment", Vec::new(), Vec::new());
		root.push_spread_items(items);
		Self::lower(vec![BsxNode::Element(root)], parsers)?
			.body
			.entities
			.swap_remove(&EntityKey::ROOT)
			.unwrap_or_default()
			.xok()
	}

	/// A document invoking the template `name` names and nothing else, its
	/// root the document rather than what the template builds.
	///
	/// ## Errors
	/// Errors when `name` is no registered template, where markup would hold
	/// the tag unregistered.
	pub fn invoking(world: &World, name: &str) -> Result<SceneBody> {
		let parsers = DocParsers::of(world);
		let registered =
			DynamicTemplate::registration(&parsers.types().read(), name)
				.is_some_and(|registration| {
					registration.data::<ReflectTemplate>().is_some()
				});
		if !registered {
			bevybail!("no template named `{name}` is registered");
		}
		Self::lower_container(
			vec![BsxNode::Element(BsxElement::new(
				name,
				Vec::new(),
				Vec::new(),
			))],
			EntityBody::default(),
			&parsers,
		)?
		.body
		.xok()
	}

	/// Spawn `markup`, a `.bsx` document ([`body`](Self::body)), answering
	/// its root.
	pub fn spawn(world: &mut World, markup: &str) -> Result<Entity> {
		let body = Self::body(world, markup)?;
		world.spawn_template(body)?.id().xok()
	}

	/// `markup` as a `.bsx` document, lowered as `world`'s registry reads it.
	pub fn body(world: &World, markup: &str) -> Result<SceneBody> {
		let bytes = MediaBytes::new_bsx(markup);
		match Self.parse(&bytes, &DocParsers::of(world)) {
			Ok(document) => document.body.xok(),
			Err(error) => Err(error.locate(&bytes, None).into()),
		}
	}
}

impl DocParser for BsxParser {
	fn media_types(&self) -> Vec<MediaType> {
		BsxParseConfig::MEDIA_TYPES.to_vec()
	}

	fn parse(
		&mut self,
		bytes: &MediaBytes,
		parsers: &DocParsers,
	) -> Result<ParsedDocument, ParseError> {
		let media_type = bytes.media_type();
		let config =
			BsxParseConfig::from_media_type(media_type).ok_or_else(|| {
				ParseError::UnsupportedType {
					unsupported: media_type.clone(),
					supported: BsxParseConfig::MEDIA_TYPES.to_vec(),
				}
			})?;
		let nodes = BsxNode::parse_document(bytes.as_utf8()?, &config)?;
		// xml resolves nothing: every element is markup as written
		match config.is_xml() {
			true => Self::lower_markup(&nodes),
			false => Self::lower(nodes, parsers),
		}?
		.xok()
	}
}

impl Installer for BsxParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}
