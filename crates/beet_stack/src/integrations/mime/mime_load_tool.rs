//! Format-aware file loading into ECS.
//!
//! [`MimeFileContent`] replaces the markdown-only [`FileContent`] with
//! a format-detecting variant that picks a parser based on the file
//! extension:
//!
//! - `.md` / `.markdown` → [`MarkdownDiffer`] (requires `markdown` feature)
//! - `.html` / `.htm` → [`HtmlDiffer`]
//! - `.json` → deserialized via [`SceneLoader`] (requires `bevy_scene` + `json`)
//! - `.postcard` → deserialized via [`SceneLoader`] (requires `bevy_scene` + `postcard`)
//!
//! Unknown extensions are treated as plain text wrapped in a
//! [`TextNode`].
//!
//! # Usage
//!
//! ```no_run
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = World::new();
//! world.spawn(MimeFileContent::new("content/home.html"));
//! ```
use crate::prelude::*;
use beet_core::prelude::*;


/// Component indicating that an entity's content should be loaded
/// from a file, with the parser chosen by the file extension.
///
/// When this component is added or changed, the
/// [`load_mime_file_content`] system reads the file, detects the
/// format, parses it, and spawns the parsed tree as children.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
pub struct MimeFileContent {
	/// Workspace-relative path to the file.
	pub path: WsPathBuf,
}

impl MimeFileContent {
	/// Create a new file content component pointing at the given path.
	pub fn new(path: impl Into<WsPathBuf>) -> Self {
		Self { path: path.into() }
	}
}

/// The detected file format for parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileFormat {
	Markdown,
	Html,
	Json,
	Postcard,
	PlainText,
}

/// Detect the file format from a path's extension.
fn detect_format(path: &WsPathBuf) -> FileFormat {
	let ext = path
		.extension()
		.map(|ext| ext.to_string_lossy().to_ascii_lowercase())
		.unwrap_or_default();
	match ext.as_str() {
		"md" | "markdown" | "mdx" => FileFormat::Markdown,
		"html" | "htm" => FileFormat::Html,
		"json" => FileFormat::Json,
		"postcard" => FileFormat::Postcard,
		_ => FileFormat::PlainText,
	}
}

/// Diff file content against an entity depending on format.
///
/// For text-based formats (markdown, HTML, plain text), parses the
/// content and diffs the resulting tree into the entity's children.
///
/// For serialized formats (JSON, postcard), deserializes the scene
/// via [`SceneLoader`] and reparents all spawned roots under the entity.
fn diff_by_format(
	format: FileFormat,
	bytes: &[u8],
	entity: EntityWorldMut,
) -> Result {
	match format {
		FileFormat::PlainText => diff_plaintext(bytes, entity),
		FileFormat::Markdown => {
			#[cfg(feature = "markdown")]
			{
				let text = std::str::from_utf8(bytes)?;
				MarkdownDiffer::new(text).diff(entity)
			}
			#[cfg(not(feature = "markdown"))]
			{
				let _ = (bytes, entity);
				bevybail!(
					"Markdown format detected but the `markdown` feature is not enabled"
				);
			}
		}
		FileFormat::Html => {
			let text = std::str::from_utf8(bytes)?;
			HtmlDiffer::new(text).diff(entity)
		}
		#[cfg(all(feature = "bevy_scene", feature = "json"))]
		FileFormat::Json => {
			let target = entity.id();
			let world = entity.into_world_mut();
			SceneLoader::new(world).with_entity(target).load_json(bytes)
		}
		#[cfg(not(all(feature = "bevy_scene", feature = "json")))]
		FileFormat::Json => {
			let _ = (bytes, entity);
			bevybail!(
				"JSON scene format requires the `bevy_scene` and `json` features"
			);
		}
		#[cfg(all(feature = "bevy_scene", feature = "postcard"))]
		FileFormat::Postcard => {
			let target = entity.id();
			let world = entity.into_world_mut();
			SceneLoader::new(world)
				.with_entity(target)
				.load_postcard(bytes)
		}
		#[cfg(not(all(feature = "bevy_scene", feature = "postcard")))]
		FileFormat::Postcard => {
			let _ = (bytes, entity);
			bevybail!(
				"Postcard scene format requires the `bevy_scene` and `postcard` features"
			);
		}
	}
}


fn diff_plaintext(bytes: &[u8], mut entity: EntityWorldMut) -> Result {
	entity.insert(TextNode::new(std::str::from_utf8(bytes)?));
	// TODO proper plaintext diff
	Ok(())
}

/// System that loads and parses files for entities with a new or
/// changed [`MimeFileContent`] component.
///
/// Reads the file from disk using [`fs_ext::read`], detects the
/// format from the extension, and applies the appropriate parser
/// as children. Binary formats (postcard) are read as raw bytes;
/// text formats are read as UTF-8.
pub fn load_mime_file_content(world: &mut World) {
	let mut to_load: Vec<(Entity, WsPathBuf)> = Vec::new();

	let mut query = world
		.query_filtered::<(Entity, &MimeFileContent), Changed<MimeFileContent>>(
		);
	for (entity, file_content) in query.iter(world) {
		to_load.push((entity, file_content.path.clone()));
	}

	for (entity, path) in to_load {
		let format = detect_format(&path);
		match fs_ext::read(&path.into_abs()) {
			Ok(bytes) => {
				if let Err(err) =
					diff_by_format(format, &bytes, world.entity_mut(entity))
				{
					cross_log_error!(
						"Failed to diff file {path} as {format:?}: {err}"
					);
				}
			}
			Err(err) => {
				cross_log_error!("Failed to load file {path}: {err}");
			}
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn detect_markdown() {
		detect_format(&WsPathBuf::new("foo.md")).xpect_eq(FileFormat::Markdown);
		detect_format(&WsPathBuf::new("foo.markdown"))
			.xpect_eq(FileFormat::Markdown);
		detect_format(&WsPathBuf::new("foo.mdx"))
			.xpect_eq(FileFormat::Markdown);
	}

	#[test]
	fn detect_html() {
		detect_format(&WsPathBuf::new("foo.html")).xpect_eq(FileFormat::Html);
		detect_format(&WsPathBuf::new("foo.htm")).xpect_eq(FileFormat::Html);
	}

	#[test]
	fn detect_json() {
		detect_format(&WsPathBuf::new("foo.json")).xpect_eq(FileFormat::Json);
	}

	#[test]
	fn detect_postcard() {
		detect_format(&WsPathBuf::new("foo.postcard"))
			.xpect_eq(FileFormat::Postcard);
	}

	#[test]
	fn detect_plain_text() {
		detect_format(&WsPathBuf::new("foo.txt"))
			.xpect_eq(FileFormat::PlainText);
		detect_format(&WsPathBuf::new("foo.unknown"))
			.xpect_eq(FileFormat::PlainText);
	}

	#[test]
	fn detect_case_insensitive() {
		detect_format(&WsPathBuf::new("foo.MD")).xpect_eq(FileFormat::Markdown);
		detect_format(&WsPathBuf::new("foo.HTML")).xpect_eq(FileFormat::Html);
		detect_format(&WsPathBuf::new("foo.JSON")).xpect_eq(FileFormat::Json);
	}

	#[test]
	#[cfg(feature = "markdown")]
	fn diff_markdown_format() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		diff_by_format(
			FileFormat::Markdown,
			b"# Hello\n\nworld",
			world.entity_mut(root),
		)
		.unwrap();

		let children: Vec<Entity> = world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		children.len().xpect_eq(2);
		world
			.entity(children[0])
			.contains::<Heading1>()
			.xpect_true();
		world
			.entity(children[1])
			.contains::<Paragraph>()
			.xpect_true();
	}

	#[test]
	fn diff_html_format() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		diff_by_format(
			FileFormat::Html,
			b"<h1>Hello</h1><p>world</p>",
			world.entity_mut(root),
		)
		.unwrap();

		let children: Vec<Entity> = world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		children.len().xpect_eq(2);
		world
			.entity(children[0])
			.contains::<Heading1>()
			.xpect_true();
		world
			.entity(children[1])
			.contains::<Paragraph>()
			.xpect_true();
	}

	#[test]
	fn diff_plain_text_format() {
		let mut world = World::new();
		let root = world.spawn_empty().id();
		diff_by_format(
			FileFormat::PlainText,
			b"just some text",
			world.entity_mut(root),
		)
		.unwrap();

		world
			.entity(root)
			.get::<TextNode>()
			.unwrap()
			.0
			.as_str()
			.xpect_eq("just some text");
	}

	/// Helper to create a world with the minimum plugins for scene
	/// serialization round-trips.
	#[cfg(feature = "bevy_scene")]
	fn scene_world() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.register_type::<Name>();
		app.init();
		app.update();
		app
	}

	#[test]
	#[cfg(all(feature = "bevy_scene", feature = "json"))]
	fn diff_json_format() {
		let mut app = scene_world();
		// Build a scene containing a named entity
		let child = app.world_mut().spawn(Name::new("JsonChild")).id();
		let scene_bytes = SceneSaver::new(app.world_mut())
			.with_entities([child])
			.save_json()
			.unwrap();

		// Load into a fresh world with a target entity
		let mut app2 = scene_world();
		let root = app2.world_mut().spawn_empty().id();
		diff_by_format(
			FileFormat::Json,
			&scene_bytes,
			app2.world_mut().entity_mut(root),
		)
		.unwrap();

		let children: Vec<Entity> = app2
			.world()
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		children.len().xpect_eq(1);
		app2.world()
			.entity(children[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("JsonChild");
	}

	#[test]
	#[cfg(all(feature = "bevy_scene", feature = "postcard"))]
	fn diff_postcard_format() {
		let mut app = scene_world();
		let child = app.world_mut().spawn(Name::new("PostcardChild")).id();
		let scene_bytes = SceneSaver::new(app.world_mut())
			.with_entities([child])
			.save_postcard()
			.unwrap();

		let mut app2 = scene_world();
		let root = app2.world_mut().spawn_empty().id();
		diff_by_format(
			FileFormat::Postcard,
			&scene_bytes,
			app2.world_mut().entity_mut(root),
		)
		.unwrap();

		let children: Vec<Entity> = app2
			.world()
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		children.len().xpect_eq(1);
		app2.world()
			.entity(children[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("PostcardChild");
	}
}
