use base64::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Request;
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

pub fn content_bundle(
	session: Entity,
	owner: Entity,
	content: impl Bundle,
) -> impl Bundle {
	(ChildOf(session), ContentOwner(owner), content)
}

/// Point to the owner of this content.
#[derive(Deref, Component)]
#[relationship(relationship_target = OwnedContent)]
pub struct ContentOwner(pub Entity);

/// List of content owned by the developer, user, or agent.
/// This is non-linked so the owner may be removed but the content
/// remains, ie somebody leaving a chat session.
#[derive(Deref, Component)]
#[relationship_target(relationship = ContentOwner)]
pub struct OwnedContent(Vec<Entity>);


/// Event notifying session members the content has ended
// TODO bevy 0.17 shouldnt need this, we have original entity
#[derive(Clone, Event)]
pub struct ContentBroadcast<E> {
	pub content: Entity,
	pub session: Entity,
	pub owner: Entity,
	pub event: E,
}



/// Emitted on a piece of content like a TextContent to indicate it has started.
/// This event does not contain text.
#[derive(Clone, Event)]
pub struct ContentAdded;
/// Emitted on a piece of content like a TextContent to indicate a new piece of text
/// was added.
#[derive(Clone, Event)]
pub struct ContentTextDelta(pub String);


impl ContentTextDelta {
	pub fn new(text: impl AsRef<str>) -> Self {
		Self(text.as_ref().to_string())
	}
}
/// Emitted on a piece of content like a TextContent to indicate it has finished
/// streaming.
#[derive(Clone, Event)]
pub struct ContentEnded;

#[derive(Event)]
pub struct ResponseComplete;


#[derive(Debug, Clone, Component)]
#[component(on_add=on_add_content)]
pub struct FileContent {
	/// The mime type of the data, for example `image/png` or `text/plain`
	pub mime_type: String,
	/// The data encoded as a base64 string
	pub data: FileData,
}

#[derive(Debug, Clone)]
pub enum FileData {
	Base64(String),
	Uri(String),
}
impl FileData {
	pub fn new_uri(uri: impl AsRef<str>) -> Self {
		Self::Uri(uri.as_ref().to_string())
	}
	pub fn new_from_bytes(data: impl AsRef<[u8]>) -> Self {
		let base_64 = BASE64_STANDARD.encode(data.as_ref());
		Self::Base64(base_64)
	}

	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub async fn new(path: impl AsRef<str>) -> Result<Self> {
		let path = path.as_ref();
		// If it's a url or already a data: url, keep as Uri
		if is_uri(path) {
			Ok(Self::new_uri(path))
		} else {
			ReadFile::to_bytes_async(path)
				.await?
				.xmap(Self::new_from_bytes)
				.xok()
		}
	}
	pub async fn get(&self) -> Result<Vec<u8>> {
		match self {
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
		write!(f, "{} ({})", self.mime_type, self.data)
	}
}

impl std::fmt::Display for FileData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			FileData::Base64(b64) => {
				write!(f, "base64:{}", &b64[..16.min(b64.len())])
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


impl FileContent {
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub async fn new(path: impl AsRef<str>) -> Result<Self> {
		let path = path.as_ref();
		let mime_type = mime_guess::from_path(path)
			.first_or_octet_stream()
			.essence_str()
			.to_string();
		let data = FileData::new(path).await?;
		Ok(Self { mime_type, data })
	}

	pub fn is_image(&self) -> bool { self.mime_type.starts_with("image/") }

	/// Returns the file url, or creates a base64 data url
	pub fn into_url(&self) -> String {
		match &self.data {
			FileData::Base64(base_64) => {
				format!("data:{};base64,{}", self.mime_type, base_64)
			}
			FileData::Uri(uri) => uri.clone(),
		}
	}
}

fn on_add_content(mut world: DeferredWorld, cx: HookContext) {
	let mut commands = world.commands();
	let mut entity = commands.entity(cx.entity);
	entity.trigger(ContentAdded);
}


#[derive(Default, Deref, DerefMut, Component)]
#[component(on_add=on_add_text)]
pub struct TextContent(pub String);

fn on_add_text(mut world: DeferredWorld, cx: HookContext) {
	let initial_text = world
		.entity(cx.entity)
		.get::<TextContent>()
		.unwrap()
		.0
		.clone();
	let mut commands = world.commands();
	let mut entity = commands.entity(cx.entity);

	entity.trigger(ContentAdded);
	if !initial_text.is_empty() {
		entity.trigger(ContentTextDelta::new(initial_text));
	}
	entity.insert(EntityObserver::new(
		|delta: Trigger<ContentTextDelta>,
		 mut text_content: Query<&mut TextContent>|
		 -> Result {
			text_content.get_mut(delta.target())?.0.push_str(&delta.0);
			Ok(())
		},
	));
}
impl TextContent {
	pub fn new(text: impl AsRef<str>) -> Self {
		TextContent(text.as_ref().to_string())
	}
}
