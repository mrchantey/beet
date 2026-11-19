use crate::prelude::*;
use beet_core::prelude::*;

pub type PostId = Uuid7<Post>;

/// What a post is for, beside who wrote it, its author's [`ActorKind`]: a
/// System actor's [`Message`](Self::Message) is a system prompt, its
/// [`Log`](Self::Log) a session divider.
#[derive(
	Debug,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	Hash,
	Serialize,
	Deserialize,
	Reflect,
)]
#[reflect(Default, Serialize, Deserialize)]
pub enum PostIntent {
	/// The conversation: text, a tool call or its output, an attachment.
	#[default]
	Message,
	/// A model's reasoning content.
	Reasoning,
	/// A summary of a model's reasoning.
	ReasoningSummary,
	/// The model refused the request.
	Refusal,
	/// The response failed.
	Error,
	/// What happened to the thread rather than what was said in it, never
	/// model input, its body naming its kind ([`LogKind`]).
	Log,
}

impl PostIntent {
	/// Whether this intent should usually be surfaced to the user.
	pub const fn is_display(&self) -> bool {
		matches!(self, Self::Message | Self::Refusal)
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
///
/// Posts implement [`Ord`], sorted by [`Self::created`]
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
	///
	/// This field is serialized as a string when valid utf-8
	/// for byte efficiency and human readability in text formats.
	#[serde(
		serialize_with = "serde_ext::serialize_bytes_or_string",
		deserialize_with = "serde_ext::deserialize_bytes_or_string"
	)]
	body: Vec<u8>,
	/// Extensible key-value metadata.
	metadata: Map,
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

impl PartialOrd for Post {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Post {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.created.cmp(&other.created)
	}
}

impl ThreadRecord for Post {
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
	pub fn metadata(&self) -> &Map { &self.metadata }
	/// Returns a mutable reference to the metadata.
	pub fn metadata_mut(&mut self) -> &mut Map { &mut self.metadata }

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

	/// Returns the `file_stem` from metadata, if present.
	pub fn file_stem(&self) -> Option<&str> {
		self.metadata
			.get("file_stem")
			.and_then(|val| val.as_str())
			.ok()
	}

	/// Hash self, used for change detection during streaming.
	pub fn hash_self(&self) -> u64 {
		use std::hash::Hash;
		use std::hash::Hasher;
		let mut hasher = FixedHasher.build_hasher();
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

	/// Set the media type, eg when replacing an image body with a text stub so the
	/// mappers send it as text rather than a base64 image.
	pub fn set_media_type(&mut self, media_type: MediaType) {
		self.media_type = media_type;
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
		metadata: Map,
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

	/// Author a seed post in markup as a child of its `<CreateActor>`, ie
	/// `children![Post::spawn("hello")]`. The author-to-behavior reduction
	/// resolves its author (the actor parent) and thread, hoists it into the
	/// [`ThreadWindow`], and despawns the entity, leaving only the record.
	///
	/// The intent follows the content (text is [`PostIntent::Message`]); see
	/// [`Self::spawn_with`] / `<CreatePost>` to author refusals, reasoning, or
	/// in-progress posts.
	pub fn spawn(content: impl Into<IntoPost>) -> SeedPost {
		SeedPost {
			content: content.into(),
			intent: PostIntent::Message,
			status: PostStatus::Completed,
		}
	}

	/// As [`Self::spawn`], overriding the [`PostIntent`] and [`PostStatus`].
	/// This is the resolver `<CreatePost>` wraps.
	pub fn spawn_with(
		content: impl Into<IntoPost>,
		intent: PostIntent,
		status: PostStatus,
	) -> SeedPost {
		SeedPost {
			content: content.into(),
			intent,
			status,
		}
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
/// Used by spawners like [`Post::spawn`] (via [`SeedPost`]).
/// Prefer [`AgentPost`] constructors for full control over intent and status.
#[derive(Clone)]
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
