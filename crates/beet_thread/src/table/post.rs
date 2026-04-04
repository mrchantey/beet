use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

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
	Reflect,
)]
#[reflect(Serialize, Deserialize)]
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
/// The body is stored as untyped bytes. Use [`Post::body_str`]
/// for text-based content, or the view types in [`AgentPost`]
/// for structured access.
///
/// Note that `MessageRole` is not stored
/// as this is relative to the Actor.
#[derive(
	Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Component,
)]
#[reflect(Serialize, Deserialize, Component)]
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
	metadata: JsonMap,
}
impl std::fmt::Debug for Post {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Post")
			.field("id", &self.id)
			.field("created", &self.created)
			.field("author", &self.author)
			.field("thread", &self.thread)
			.field("intent", &self.intent)
			.field("media_type", &self.media_type)
			.field("body", &match self.body_str() {
				Ok(s) => s.to_string(),
				Err(_) => format!("{} bytes", self.body.len()),
			})
			.field("metadata", &self.metadata)
			.finish()
	}
}


impl Table for Post {
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
	pub fn body_str(&self) -> Result<&str> {
		std::str::from_utf8(&self.body)?.xok()
	}
	pub fn body_string(self) -> Result<String> {
		Ok(String::from_utf8(self.body)?)
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
		if let Ok(text) = self.body_str() {
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
	/// Low-level constructor. Prefer the [`AgentPost`] constructors,
	/// ie [`AgentPost::new_text`], [`AgentPost::new_function_call`], etc.
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
			metadata: JsonMap(metadata),
		}
	}

	/// For a given body, resolve the author id and thread id
	/// on spawn by recursing up the tree.
	pub fn spawn(content: impl Into<IntoPost>) -> OnSpawn {
		let content = content.into();
		OnSpawn::new(move |entity| {
			let post = entity.with_state::<ThreadQuery, _>(
				move |post_entity, query| -> Result<Post> {
					let thread = query.thread(post_entity)?;
					let actor = query.actor_from_post_entity(post_entity)?;
					content.into_post(actor.id(), thread.id()).xok()
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

// ═══════════════════════════════════════════════════════════════════════
// IntoPost
// ═══════════════════════════════════════════════════════════════════════

/// Content for constructing a new [`Post`].
///
/// Used by spawners like [`Post::spawn`] and [`ThreadQuery::spawn_post`].
/// Prefer [`AgentPost`] constructors for full control over intent and status.
pub enum IntoPost {
	Text(String),
	Url {
		url: String,
		file_stem: Option<String>,
	},
	Bytes {
		media_type: MediaType,
		bytes: Vec<u8>,
		file_stem: Option<String>,
	},
	FunctionCall {
		name: String,
		call_id: String,
		arguments: String,
	},
	FunctionCallOutput {
		call_id: String,
		output: String,
		name: Option<String>,
	},
}

impl IntoPost {
	/// Converts into a completed [`Post`] with the given author and thread.
	pub fn into_post(self, author: ActorId, thread: ThreadId) -> Post {
		let status = PostStatus::Completed;
		match self {
			IntoPost::Text(text) => {
				AgentPost::new_text(author, thread, text, status)
			}
			IntoPost::Url { url, file_stem } => {
				AgentPost::new_url(author, thread, url, file_stem, status)
			}
			IntoPost::Bytes {
				media_type,
				bytes,
				file_stem,
			} => AgentPost::new_bytes(
				author, thread, media_type, bytes, file_stem, status,
			),
			IntoPost::FunctionCall {
				name,
				call_id,
				arguments,
			} => AgentPost::new_function_call(
				author, thread, name, call_id, arguments, status,
			),
			IntoPost::FunctionCallOutput {
				call_id,
				output,
				name,
			} => AgentPost::new_function_call_output(
				author, thread, call_id, output, name, status,
			),
		}
	}
}

impl From<String> for IntoPost {
	fn from(text: String) -> Self { IntoPost::Text(text) }
}

impl From<&str> for IntoPost {
	fn from(text: &str) -> Self { IntoPost::Text(text.to_string()) }
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
	Reflect,
)]
#[reflect(Serialize, Deserialize)]
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
