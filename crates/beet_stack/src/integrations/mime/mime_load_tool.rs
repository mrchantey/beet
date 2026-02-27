//! Format-aware file loading into ECS.
//!
//! [`MimeFileContent`] replaces the markdown-only [`FileContent`] with
//! a format-detecting variant that picks a parser based on the file
//! extension:
//!
//! - `.md` / `.markdown` → [`MarkdownDiffer`]
//! - `.html` / `.htm` → [`HtmlDiffer`]
//! - `.json` → deserialized via [`mime_serde`] (JSON)
//! - `.postcard` → deserialized via [`mime_serde`] (postcard binary)
//!
//! Unknown extensions are treated as plain text wrapped in a
//! [`Paragraph`].
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
/// For serialized formats (JSON, postcard), deserializes the content
/// as a markdown string via [`mime_serde`], then diffs as markdown.
fn diff_by_format(
	format: FileFormat,
	bytes: &[u8],
	entity: EntityWorldMut,
) -> Result {
	match format {
		FileFormat::Markdown => {
			let text = std::str::from_utf8(bytes)?;
			MarkdownDiffer::new(text).diff(entity)
		}
		FileFormat::Html => {
			let text = std::str::from_utf8(bytes)?;
			HtmlDiffer::new(text).diff(entity)
		}
		FileFormat::Json => {
			let text: String = mime_serde::deserialize(MimeType::Json, bytes)?;
			MarkdownDiffer::new(&text).diff(entity)
		}
		FileFormat::Postcard => {
			let text: String =
				mime_serde::deserialize(MimeType::Postcard, bytes)?;
			MarkdownDiffer::new(&text).diff(entity)
		}
		FileFormat::PlainText => {
			let text = std::str::from_utf8(bytes)?;
			// Wrap plain text in a paragraph
			let wrapped = format!("<p>{}</p>", text);
			HtmlDiffer::new(&wrapped).diff(entity)
		}
	}
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

		let children: Vec<Entity> = world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		children.len().xpect_eq(1);
		world
			.entity(children[0])
			.contains::<Paragraph>()
			.xpect_true();
	}

	#[test]
	fn diff_json_format() {
		let markdown = "# Hello\n\nworld";
		let json_bytes =
			mime_serde::serialize(MimeType::Json, &markdown).unwrap();
		let mut world = World::new();
		let root = world.spawn_empty().id();
		diff_by_format(FileFormat::Json, &json_bytes, world.entity_mut(root))
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
	fn diff_postcard_format() {
		let markdown = "# Hello\n\nworld";
		let postcard_bytes =
			mime_serde::serialize(MimeType::Postcard, &markdown).unwrap();
		let mut world = World::new();
		let root = world.spawn_empty().id();
		diff_by_format(
			FileFormat::Postcard,
			&postcard_bytes,
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
}
