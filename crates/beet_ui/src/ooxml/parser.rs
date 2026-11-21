use super::deck::*;
use super::node::OoxmlRead;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;

/// Parses a slide deck into the one document tree: the [`DocMeta`] its core
/// properties declare on the root, each part it reads an
/// [`OoxmlNode::Part`] beneath, every node of the part an entity with its
/// [`OoxmlNode`] and, where it means what an HTML element means, that
/// [`Element`].
#[derive(Debug, Default, Clone)]
pub struct OoxmlParser;

impl OoxmlParser {
	/// The media types this parser reads, a Word file being the
	/// [`WordParser`]'s and a workbook the [`WorkbookParser`]'s.
	const MEDIA_TYPES: [MediaType; 1] = [MediaType::Pptx];

	/// Reads `part` of `package` as an [`OoxmlNode::Part`] child of `root`,
	/// holding the part's prolog and document element.
	pub(crate) fn spawn_part(
		world: &mut World,
		root: Entity,
		package: &OoxmlFile,
		part: &PartRef,
	) -> Result<Entity> {
		let path = package
			.path(part)
			.ok_or_else(|| bevyhow!("a related part has no path"))?;
		let bytes = package
			.data(part)?
			.ok_or_else(|| bevyhow!("the part `{path}` is empty"))?;
		let nodes = OoxmlFile::read_xml(bytes)?;
		let entity = OoxmlNode::spawn_part(world, path, &nodes)?;
		world.entity_mut(root).add_child(entity);
		entity.xok()
	}
}

impl Installer for OoxmlParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

impl DocParser for OoxmlParser {
	fn media_types(&self) -> Vec<MediaType> { Self::MEDIA_TYPES.to_vec() }

	/// The [`DocMeta`] a file's core properties declare, named as
	/// `frontmatter_type`.
	fn declarations(
		&self,
		bytes: &MediaBytes,
		frontmatter_type: &str,
	) -> Result<RootDeclarations> {
		OoxmlFile::open(bytes)?
			.core_properties()?
			.declarations(frontmatter_type)
			.xok()
	}

	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		if !Self::MEDIA_TYPES.contains(cx.bytes.media_type()) {
			return Err(ParseError::UnsupportedType {
				unsupported: cx.bytes.media_type().clone(),
				supported: Self::MEDIA_TYPES.to_vec(),
			});
		}
		let file = OoxmlFile::open(cx.bytes)?;
		let root = cx.entity.id();
		let properties = file.core_properties()?;
		cx.entity.insert((properties.doc_meta(), properties));
		cx.entity.world_scope(|world| -> Result {
			SlideProjection::build(world, root, &file)?;
			// the read ends here: a value differing from it is an edit
			OoxmlRead::record(world, root);
			Ok(())
		})?;
		Ok(())
	}
}
