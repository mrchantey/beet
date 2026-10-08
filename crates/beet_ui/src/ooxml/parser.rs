use super::deck::*;
use super::word::*;
use super::workbook::*;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;

/// Parses a Word file, a workbook or a slide deck into the one document tree:
/// the [`PageMeta`] its core properties declare and its [`OoxmlPackage`] on
/// the root, each part it reads a [`SourcePart`] beneath, every node of the
/// part an entity with its source identity and, where it means what an HTML
/// element means, that [`Element`]. See the [`ooxml`](crate::ooxml) module
/// for what each format's nodes become.
#[derive(Debug, Default, Clone)]
pub struct OoxmlParser;

impl OoxmlParser {
	/// The media types this parser reads.
	pub const SUPPORTED: [MediaType; 3] = OoxmlPackage::MEDIA_TYPES;

	/// The [`PageMeta`] a file's core properties declare, named as
	/// `component`, for a scan that reads a root without building it.
	pub fn declarations(
		bytes: &MediaBytes,
		component: &str,
	) -> Result<RootDeclarations> {
		OoxmlFile::open(bytes)?
			.core_properties()?
			.declarations(component)
			.xok()
	}

	/// Reads `part` of `package` as a source subtree: a [`SourcePart`]
	/// child of `root` holding the part's prolog and document element.
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
		let entity = world.spawn((SourcePart::new(path), ChildOf(root))).id();
		BsxNode::spawn_source(&nodes, &mut world.entity_mut(entity));
		entity.xok()
	}
}

impl NodeParser for OoxmlParser {
	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		if !Self::SUPPORTED.contains(cx.bytes.media_type()) {
			return Err(ParseError::UnsupportedType {
				unsupported: cx.bytes.media_type().clone(),
				supported: Self::SUPPORTED.to_vec(),
			});
		}
		let file = OoxmlFile::open(cx.bytes)?;
		let root = cx.entity.id();
		cx.entity.insert(file.core_properties()?.page_meta());
		cx.entity.world_scope(|world| -> Result {
			match file.media_type() {
				MediaType::Docx => WordProjection::build(world, root, &file)?,
				MediaType::Xlsx => {
					WorkbookProjection::build(world, root, &file)?
				}
				_ => SlideProjection::build(world, root, &file)?,
			}
			// the read ends here: a value changed after this tick is an edit
			let parsed = world.change_tick();
			world.increment_change_tick();
			world
				.entity_mut(root)
				.insert(OoxmlPackage::new(cx.bytes.clone(), parsed));
			// what extends a read tree, ie a deck's triage, when registered
			let _ = world.try_run_schedule(PostParseTree);
			Ok(())
		})?;
		Ok(())
	}
}
