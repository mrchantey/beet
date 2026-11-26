use crate::prelude::*;
use beet_core::prelude::*;
use beet_scene::prelude::*;

/// The Office formats as media: each format's reader and writer, and the
/// [`StylePlugin`] whose user agent rules every look reads and writes
/// through.
#[derive(Default)]
pub struct OoxmlPlugin;

impl Plugin for OoxmlPlugin {
	fn build(&self, app: &mut App) {
		app.install_types::<StylePlugin>().install((
			WordParser::default(),
			WordRenderer,
			WorkbookParser::default(),
			WorkbookRenderer,
			DeckParser::default(),
			DeckRenderer,
		));
	}
}

impl OoxmlPlugin {
	/// The package `bytes` hold, refusing any media type but `media_type`.
	pub(super) fn open(
		bytes: &MediaBytes,
		media_type: MediaType,
	) -> Result<OpcPackage, ParseError> {
		if bytes.media_type() != &media_type {
			return Err(ParseError::UnsupportedType {
				unsupported: bytes.media_type().clone(),
				supported: vec![media_type],
			});
		}
		OpcPackage::read(bytes.bytes())?.xok()
	}

	/// The document `read` builds under a root, its root declaring the
	/// metadata `package`'s core properties do, named as `parsers` names it.
	///
	/// A reader assembles and spawns a tree, so it builds in a world of its
	/// own, which knows what a read tree carries and nothing else and is never
	/// a caller's, then the tree is saved as the body.
	pub(super) fn read_body(
		package: &OpcPackage,
		parsers: &DocParsers,
		read: impl FnOnce(&mut World, Entity) -> Result,
	) -> Result<ParsedDocument, ParseError> {
		let mut world = ScenePlugin::world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<InlineRule>();
		let root = world.spawn_empty().id();
		read(&mut world, root)?;
		let (mut body, _) =
			TemplateSaver::new().roots_body(&mut world, &[root], default())?;
		let metadata = DocMeta::from_package(package)?.components(parsers)?;
		if let Some((_, root)) = body.entities.first_mut() {
			for (name, value) in Map::from(metadata) {
				root.insert(name, value);
			}
		}
		ParsedDocument::from(body).xok()
	}
}
