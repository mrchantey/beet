use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;

pub type PostId = Uuid7<Post>;

/// Analogous to a simplified HTTP status code, declaring the intent of the post.
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
)]
pub struct PostIntent(u16);

impl PostIntent {
	/// 1xx informational: reasoning content from a model.
	pub const REASONING_CONTENT: PostIntent = PostIntent(102);
	/// 1xx informational: reasoning summary from a model.
	pub const REASONING_SUMMARY: PostIntent = PostIntent(103);
	/// 2xx success: a successful response to be surfaced to the user.
	pub const OK: PostIntent = PostIntent(200);
	/// 4xx client error: the model refused the request.
	pub const REFUSAL: PostIntent = PostIntent(403);
	/// 5xx server error: an internal error occurred.
	pub const INTERNAL_ERROR: PostIntent = PostIntent(500);

	/// Creates a new [`PostIntent`] with the given code.
	pub const fn new(code: u16) -> Self { Self(code) }

	/// Returns the raw numeric code.
	pub const fn code(&self) -> u16 { self.0 }

	/// Returns `true` if this is a 1xx informational status.
	pub const fn is_informational(&self) -> bool {
		self.0 >= 100 && self.0 < 200
	}

	/// Returns `true` if this is a 2xx success status.
	pub const fn is_success(&self) -> bool { self.0 >= 200 && self.0 < 300 }

	/// Returns `true` if this is a 4xx client error status.
	pub const fn is_client_error(&self) -> bool {
		self.0 >= 400 && self.0 < 500
	}

	/// Returns `true` if this is a 5xx server error status.
	pub const fn is_server_error(&self) -> bool {
		self.0 >= 500 && self.0 < 600
	}

	/// Whether this intent should usually be surfaced to the user.
	pub const fn is_display(&self) -> bool {
		self.is_success() || self.is_client_error()
	}
}

impl std::fmt::Display for PostIntent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match *self {
			Self::REASONING_CONTENT => write!(f, "102 Reasoning Content"),
			Self::REASONING_SUMMARY => write!(f, "103 Reasoning Summary"),
			Self::OK => write!(f, "200 OK"),
			Self::REFUSAL => write!(f, "403 Refusal"),
			Self::INTERNAL_ERROR => write!(f, "500 Internal Error"),
			other => write!(f, "{}", other.0),
		}
	}
}

/// A post by an actor on a thread.
///
/// The body is stored as untyped bytes. Use [`Post::as_str`]
/// for text-based content, or the view types in [`AgentPost`]
/// for structured access.
///
/// Note that `MessageRole` is not stored
/// as this is relative to the Actor.
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Component,
)]
pub struct Post {
	id: PostId,
	created: Timestamp,
	/// The actor that created this post.
	author: ActorId,
	thread: ThreadId,
	intent: PostIntent,
	media_type: MediaType,
	/// The untyped body for this post.
	body: Vec<u8>,
	/// Extensible key-value metadata.
	metadata: serde_json::Map<String, serde_json::Value>,
}

impl Document for Post {
	type Id = PostId;
	fn id(&self) -> Self::Id { self.id }
}

// ═══════════════════════════════════════════════════════════════════════
// Core accessors
// ═══════════════════════════════════════════════════════════════════════

impl Post {
	pub fn author(&self) -> ActorId { self.author }
	pub fn thread(&self) -> ThreadId { self.thread }
	pub fn created(&self) -> Timestamp { self.created }
	pub fn intent(&self) -> PostIntent { self.intent }
	pub fn media_type(&self) -> &MediaType { &self.media_type }
	pub fn body_bytes(&self) -> &[u8] { &self.body }
	pub fn metadata(&self) -> &serde_json::Map<String, serde_json::Value> {
		&self.metadata
	}

	pub fn set_intent(&mut self, intent: PostIntent) { self.intent = intent; }

	/// Returns the body as a `&str`.
	/// ## Errors
	/// Errors if the body is not valid utf-8.
	pub fn as_str(&self) -> Result<&str> {
		std::str::from_utf8(&self.body)?.xok()
	}

	/// Returns a mutable reference to the raw body bytes.
	pub fn body_bytes_mut(&mut self) -> &mut Vec<u8> { &mut self.body }

	/// Returns a mutable reference to the metadata.
	pub fn metadata_mut(
		&mut self,
	) -> &mut serde_json::Map<String, serde_json::Value> {
		&mut self.metadata
	}

	/// Returns the `file_stem` from metadata, if present.
	pub fn file_stem(&self) -> Option<&str> {
		self.metadata.get("file_stem").and_then(|val| val.as_str())
	}

	/// Hash self, used for change detection during streaming.
	pub fn hash_self(&self) -> u64 {
		use std::hash::Hash;
		use std::hash::Hasher;
		let mut hasher = std::collections::hash_map::DefaultHasher::new();
		self.hash(&mut hasher);
		hasher.finish()
	}

	/// Append text to the body. Only valid for text-based media types.
	pub fn push_str(&mut self, text: &str) {
		self.body.extend_from_slice(text.as_bytes());
	}

	/// Replace the body with the given text.
	pub fn set_text(&mut self, text: impl Into<String>) {
		let text = text.into();
		self.body = text.into_bytes();
	}
}


// ═══════════════════════════════════════════════════════════════════════
// Display
// ═══════════════════════════════════════════════════════════════════════

impl std::fmt::Display for Post {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Ok(text) = self.as_str() {
			write!(f, "{}", text)
		} else {
			write!(f, "[{} body, {} bytes]", self.media_type, self.body.len())
		}
	}
}

// ═══════════════════════════════════════════════════════════════════════
// Constructor
// ═══════════════════════════════════════════════════════════════════════

impl Post {
	/// Low-level constructor. Prefer the typed view constructors,
	/// ie [`TextView::into_post`], [`FunctionCallView::into_post`], etc.
	pub fn new_raw(
		author: ActorId,
		thread: ThreadId,
		intent: PostIntent,
		media_type: MediaType,
		body: Vec<u8>,
		metadata: serde_json::Map<String, serde_json::Value>,
	) -> Self {
		Self {
			id: Uuid7::new_now(),
			created: Timestamp::now(),
			author,
			thread,
			intent,
			media_type,
			body,
			metadata,
		}
	}

	/// For a given body, resolve the author id and thread id
	/// on spawn by recursing up the tree.
	pub fn spawn(text: impl Into<String>) -> OnSpawn {
		let text = text.into();
		OnSpawn::new(move |entity| {
			let post = entity.with_state::<SocialQuery, _>(
				move |post_entity, query| -> Result<Post> {
					let thread = query.thread(post_entity)?;
					let actor = query.actor_from_post_entity(post_entity)?;
					Ok(TextView::into_post(actor.id(), thread.id(), text))
				},
			)?;
			entity.insert(post);
			Ok(())
		})
	}

	/// Returns the body as base64.
	pub fn body_base64(&self) -> String {
		base64::Engine::encode(
			&base64::prelude::BASE64_STANDARD,
			self.body_bytes(),
		)
	}
}

/// Compat constructor: builds a text post from `&str`.
impl From<&str> for Post {
	fn from(text: &str) -> Self {
		Post::new_raw(
			ActorId::default(),
			ThreadId::default(),
			PostIntent::OK,
			MediaType::Text,
			text.as_bytes().to_vec(),
			serde_json::Map::new(),
		)
	}
}

impl From<String> for Post {
	fn from(text: String) -> Self {
		Post::new_raw(
			ActorId::default(),
			ThreadId::default(),
			PostIntent::OK,
			MediaType::Text,
			text.into_bytes(),
			serde_json::Map::new(),
		)
	}
}

impl<'a> From<Cow<'a, String>> for Post {
	fn from(text: Cow<'a, String>) -> Self { Post::from(text.into_owned()) }
}


// ═══════════════════════════════════════════════════════════════════════
// Timestamp
// ═══════════════════════════════════════════════════════════════════════

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
)]
pub struct Timestamp(Duration);
impl Timestamp {
	pub fn now() -> Self {
		Self(
			SystemTime::now()
				.duration_since(SystemTime::UNIX_EPOCH)
				.unwrap(),
		)
	}
	pub fn as_system_time(&self) -> SystemTime {
		SystemTime::UNIX_EPOCH + self.0
	}
	pub fn unix_epoch_elapsed(&self) -> Duration { self.0 }
}

impl Default for Timestamp {
	fn default() -> Self { Self(Duration::ZERO) }
}
