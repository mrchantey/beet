use base64::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Request;
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use std::fmt::Debug;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ContentView<'a> {
	Text(&'a TextContent),
	File(&'a FileContent),
}
impl ContentView<'_> {
	pub fn as_text(&self) -> Option<&TextContent> {
		match self {
			ContentView::Text(text) => Some(text),
			_ => None,
		}
	}
	pub fn as_file(&self) -> Option<&FileContent> {
		match self {
			ContentView::File(file) => Some(file),
			_ => None,
		}
	}
}

/// Marker component indicating the root entity for an actor's message.
/// Messages must be (possibly nested) descendents of an [`Actor`], and may
/// contain Content either in its entity its descendents.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct Content {
	pub created: Timestamp,
}

#[derive(Debug, Clone, Copy, Deref)]
pub struct Timestamp(Instant);

impl Default for Timestamp {
	fn default() -> Self { Self(Instant::now()) }
}

/// Added to a [`Content`] when it is finished, and no more content
/// will be added to it.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct ContentEnded {
	pub completed: Timestamp,
}

#[derive(Default, Component)]
#[require(Content)]
pub struct ReasoningContent;


#[derive(Debug, Default, Deref, DerefMut, Component)]
#[require(Content)]
#[component(on_add=handle_text_delta)]
pub struct TextContent(pub String);

impl TextContent {
	pub fn new(text: impl AsRef<str>) -> Self {
		TextContent(text.as_ref().to_string())
	}
}

/// Emitted on a piece of content like a TextContent to indicate a new piece of text
/// was added.
#[derive(Clone, Event)]
pub struct TextDelta(pub String);


impl TextDelta {
	pub fn new(text: impl AsRef<str>) -> Self {
		Self(text.as_ref().to_string())
	}
}

fn handle_text_delta(mut world: DeferredWorld, cx: HookContext) {
	let initial_text = world
		.entity(cx.entity)
		.get::<TextContent>()
		.unwrap()
		.0
		.clone();
	let mut commands = world.commands();
	let mut entity = commands.entity(cx.entity);

	if !initial_text.is_empty() {
		entity.trigger(TextDelta::new(initial_text));
	}
	entity.insert(EntityObserver::new(
		|delta: Trigger<TextDelta>,
		 mut text_content: Query<&mut TextContent>|
		 -> Result {
			text_content.get_mut(delta.target())?.0.push_str(&delta.0);
			Ok(())
		},
	));
}



#[derive(Debug, Clone, Component)]
#[require(Content)]
pub struct FileContent {
	/// The mime type of the data, for example `image/png` or `text/plain`
	pub mime_type: String,
	/// The file path, primarily used for extracting the file name
	pub filename: PathBuf,
	/// The data encoded as a base64 string
	pub data: FileData,
}

impl FileContent {
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub async fn new(path: impl AsRef<str>) -> Result<Self> {
		let path = path.as_ref();
		let mime_type = mime_guess::from_path(path)
			.first_or_octet_stream()
			.essence_str()
			.to_string();
		let filename = PathBuf::from(path);
		let data = FileData::new(path, &mime_type).await?;
		Ok(Self {
			mime_type,
			data,
			filename,
		})
	}

	pub fn new_b64(file_stem: &str, ext: &str, b64: &str) -> Self {
		let mime_type = mime_guess::from_ext(ext)
			.first_or_octet_stream()
			.essence_str()
			.to_string();
		let filename = format!("{}.{}", file_stem, ext).into();
		Self {
			mime_type,
			filename,
			data: FileData::Base64(b64.to_string()),
		}
	}

	pub fn extension(&self) -> &str {
		self.filename.extension().unwrap().to_str().unwrap()
	}

	pub fn is_image(&self) -> bool { self.mime_type.starts_with("image/") }

	/// Returns the file url, or creates a base64 data url
	pub fn into_url(&self) -> String {
		match &self.data {
			FileData::Base64(base_64) => {
				format!("data:{};base64,{}", self.mime_type, base_64)
			}
			FileData::Utf8(utf8) => {
				format!("data:{};charset=utf-8,{}", self.mime_type, utf8)
			}
			FileData::Uri(uri) => uri.clone(),
		}
	}
}


#[derive(Debug, Clone)]
pub enum FileData {
	Utf8(String),
	Base64(String),
	Uri(String),
}
impl FileData {
	pub fn new_uri(uri: impl AsRef<str>) -> Self {
		Self::Uri(uri.as_ref().to_string())
	}

	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub async fn new(path: impl AsRef<str>, mime_type: &str) -> Result<Self> {
		let path = path.as_ref();
		// If it's a url or already a data: url, keep as Uri
		if is_uri(path) {
			Self::new_uri(path)
		} else if mime_type.starts_with("text/") {
			let bytes = ReadFile::to_bytes_async(path).await?;
			let utf8 = String::from_utf8(bytes)?;
			Self::Utf8(utf8)
		} else {
			let bytes = ReadFile::to_bytes_async(path).await?;
			let base_64 = BASE64_STANDARD.encode(bytes);
			Self::Base64(base_64)
		}
		.xok()
	}
	pub async fn get(&self) -> Result<Vec<u8>> {
		match self {
			FileData::Utf8(utf8) => Ok(utf8.as_bytes().to_vec()),
			FileData::Base64(b64) => {
				let bytes = BASE64_STANDARD.decode(b64)?;
				Ok(bytes)
			}
			FileData::Uri(uri) => {
				if uri.starts_with("data:") {
					let parts: Vec<&str> = uri.splitn(2, ",").collect();
					if parts.len() != 2 {
						bevybail!("Invalid data URL: {}", uri);
					} else if !parts[0].ends_with(";base64") {
						bevybail!(
							"Only base64-encoded data URLs are supported: {}",
							uri
						);
					} else {
						BASE64_STANDARD.decode(parts[1])?.xok()
					}
				} else if is_uri(uri) {
					Request::get(uri)
						.send()
						.await?
						.into_result()
						.await?
						.bytes()
						.await
						.map(|b| b.to_vec())
				} else {
					// assume workspace relative file path
					AbsPathBuf::new_workspace_rel(uri)?
						.xmap(ReadFile::to_bytes_async)
						.await?
						.xok()
				}
			}
		}
	}
}

impl std::fmt::Display for FileContent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} ({})", self.filename.display(), self.data)
	}
}

impl std::fmt::Display for FileData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FileData::Base64(b64) => {
				write!(f, "base64:{}", &b64[..16.min(b64.len())])
			}
			FileData::Utf8(utf8) => {
				let snippet = if utf8.len() > 16 { &utf8[..16] } else { &utf8 };
				write!(f, "utf8:{}", snippet.escape_debug())
			}
			FileData::Uri(uri) => write!(f, "uri:{}", uri),
		}
	}
}

fn is_uri(path: &str) -> bool {
	let path_lower = path.to_ascii_lowercase();
	path_lower.starts_with("http://")
		|| path_lower.starts_with("https://")
		|| path_lower.starts_with("data:")
}
