use crate::prelude::*;
use beet_core::prelude::*;

/// Reads documents as scenes and builds them: the base formats, every type
/// markup and HTML's own elements write, and the [`SceneDocument`] sync.
///
/// The formats are registered behind any already there
/// ([`DocParsers::register_base`]), so a downstream parser wins whenever it
/// was installed:
///
/// ```text
/// application/json,                       a scene body when the root object
/// application/postcard, application/toml  carries `$type: org.beet.scene#body`,
///                                         else one entity holding the `Value`
/// text/javascript                         one entity carrying `RunScript`, the
///                                         source as its `Value`
/// text/plain, any other text type         one entity holding the text
/// application/vnd.beet.bsx, text/html,    a scene body
/// application/xml
/// text/markdown, text/mdx                 a scene body (`markdown_parser`)
/// ```
///
/// An [`Instance`] builds nothing until [`SceneStorePlugin`] reads the document
/// it names.
#[derive(Default)]
pub struct ScenePlugin;

impl ScenePlugin {
	/// The base formats alone, for a caller with no app, ie a codegen scan.
	pub fn parsers() -> DocParsers {
		let mut parsers = DocParsers::default();
		Self::register_formats(&mut parsers);
		parsers
	}

	/// Register the base formats behind any `parsers` holds.
	fn register_formats(parsers: &mut DocParsers) {
		#[cfg(feature = "markdown_parser")]
		parsers
			.register_base(MarkdownParser::mdx())
			.register_base(MarkdownParser::new());
		parsers
			.register_base(BsxParser)
			.register_base(StructuredParser)
			.register_base(ScriptParser)
			.register_base(PlainTextParser::default());
	}
}

impl Plugin for ScenePlugin {
	fn build(&self, app: &mut App) {
		app.install_types::<TemplatePlugin>()
			.init_resource::<BsxConditions>()
			.insert_resource(SceneBodyBuilder::new(|body, path, cx| {
				DynamicTemplate::reflect_document(
					cx.entity.world(),
					body,
					path,
				)?
				.xmap(|template| template.build_template(cx))
			}))
			// what markup, a directive or an element of HTML's builds into,
			// resolved by name, so a document reads with nothing else installed
			.register_type::<ChildOf>()
			.register_type::<FieldRef>()
			.register_type::<SourceFieldRef>()
			.register_type::<ResourceFieldRef>()
			.register_type::<ReflectFieldRef>()
			.register_type::<DocumentScope>()
			.register_type::<ReactiveChildren>()
			.register_type::<Cfg>()
			.register_type::<CfgExcluded>()
			// the component a document's unsectioned frontmatter declares
			.register_type::<FrontmatterType>()
			// the fork relation a scene records on its first entity
			.register_type::<SceneFork>()
			.add_systems(
				DocumentSync,
				crate::body::sync_scene_documents
					.in_set(DocumentSyncSet::Apply),
			);
		Self::register_formats(
			&mut app.world_mut().get_resource_or_init::<DocParsers>(),
		);
	}
}

/// Reads the documents a scene names out of their stores: each [`Instance`]
/// activated and each [`SceneBlob`] booted and written back.
#[derive(Default)]
pub struct SceneStorePlugin;

impl Plugin for SceneStorePlugin {
	fn build(&self, app: &mut App) {
		app.install_types::<ScenePlugin>()
			.install_types::<AsyncPlugin>()
			.init_resource::<PrefabCache>()
			.add_observer(crate::instance::activate_on_insert)
			.add_observer(crate::instance::forget_changed_prefabs);
		#[cfg(feature = "json")]
		app.register_type::<SceneBlob>()
			.add_systems(PreUpdate, crate::scene_blob::read_scene_blobs)
			.add_systems(PostUpdate, crate::scene_blob::write_scene_blobs);
	}
}
