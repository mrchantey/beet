use std::borrow::Cow;

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Url;
use serde::Deserialize;
use serde::Serialize;

pub type PostId = Uuid7<Post>;

/// A post by an actor on a thread.
///
/// Note that MessageRole is not stored
/// as this is relative to the Actor.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
	Component,
)]
pub struct Post {
	id: PostId,
	/// The actor that created this post.
	author: ActorId,
	thread: ThreadId,
	status: PostStatus,
	/// For function calls this is the time the call was completed.
	created: Timestamp,
	payload: PostPayload,
}

impl Document for Post {
	type Id = PostId;
	fn id(&self) -> Self::Id { self.id }
}


impl Post {
	pub fn new(
		author: ActorId,
		thread: ThreadId,
		status: PostStatus,
		payload: impl Into<PostPayload>,
	) -> Self {
		Self {
			id: Uuid7::new_now(),
			author,
			thread,
			status,
			payload: payload.into(),
			created: Timestamp::now(),
		}
	}
	/// For a given payload, resolve the author id and thread id
	/// on spawn by recursing up the tree.
	pub fn spawn(payload: impl Into<PostPayload>) -> OnSpawn {
		let payload = payload.into();
		OnSpawn::new(move |entity| {
			let post = entity.with_state::<SocialQuery, _>(
				move |post_entity, query| -> Result<Post> {
					let thread = query.thread(post_entity)?;
					let actor = query.actor_from_post_entity(post_entity)?;
					Ok(Post::new(
						actor.id(),
						thread.id(),
						PostStatus::Completed,
						payload,
					))
				},
			)?;
			entity.insert(post);
			Ok(())
		})
	}

	pub fn author(&self) -> ActorId { self.author }
	pub fn thread(&self) -> ThreadId { self.thread }
	pub fn status(&self) -> PostStatus { self.status }
	pub fn created(&self) -> Timestamp { self.created }
	pub fn payload(&self) -> &PostPayload { &self.payload }
	pub fn set_status(&mut self, status: PostStatus) { self.status = status; }
	pub fn hash(&self) -> u64 {
		use std::hash::Hash;
		use std::hash::Hasher;
		let mut hasher = std::collections::hash_map::DefaultHasher::new();
		self.payload.hash(&mut hasher);
		hasher.finish()
	}
	pub fn payload_mut(&mut self) -> &mut PostPayload { &mut self.payload }
}

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
pub enum PostStatus {
	Completed,
	Interrupted,
	InProgress,
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PostPayload {
	Text(TextItem),
	Refusal(RefusalItem),
	ReasoningSummary(ReasoningSummaryItem),
	ReasoningContent(ReasoningContentItem),
	Url(UrlItem),
	Bytes(BytesItem),
	FunctionCall(FunctionCallItem),
	FunctionCallOutput(FunctionCallOutputItem),
}

impl PostPayload {
	pub fn kind(&self) -> PostKind {
		match self {
			Self::Text(_) => PostKind::Text,
			Self::Refusal(_) => PostKind::Refusal,
			Self::ReasoningSummary(_) => PostKind::ReasoningSummary,
			Self::ReasoningContent(_) => PostKind::ReasoningContent,
			Self::Url(_) => PostKind::Url,
			Self::Bytes(_) => PostKind::Media,
			Self::FunctionCall(_) => PostKind::FunctionCall,
			Self::FunctionCallOutput(_) => PostKind::FunctionCallOutput,
		}
	}
	pub fn to_string(&self) -> String {
		match self {
			Self::Text(text) => text.to_string(),
			Self::Refusal(refusal) => refusal.to_string(),
			Self::ReasoningSummary(reasoning_summary) => {
				reasoning_summary.to_string()
			}
			Self::ReasoningContent(reasoning_content) => {
				reasoning_content.to_string()
			}
			Self::Url(url_item) => url_item.url().to_string(),
			Self::Bytes(bytes_item) => format!(
				"BytesItem: filename={}, media_type={}, bytes_length={}",
				bytes_item.filename(),
				bytes_item.media_type(),
				bytes_item.bytes().len()
			),
			Self::FunctionCall(function_call) => format!(
				"FunctionCallItem: name={}, call_id={}, arguments={}",
				function_call.function_name(),
				function_call.args(),
				function_call.call_id()
			),
			Self::FunctionCallOutput(function_call_output) => format!(
				"FunctionCallOutputItem: call_id={}, output={}",
				function_call_output.call_id(),
				function_call_output.output()
			),
		}
	}
}

impl std::fmt::Display for PostPayload {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.to_string())
	}
}

impl From<&str> for PostPayload {
	fn from(text_content: &str) -> Self {
		Self::Text(TextItem(text_content.to_string()))
	}
}
impl From<String> for PostPayload {
	fn from(text_content: String) -> Self { Self::Text(TextItem(text_content)) }
}
impl<'a> From<Cow<'a, String>> for PostPayload {
	fn from(text_content: Cow<'a, String>) -> Self {
		Self::Text(TextItem(text_content.to_string()))
	}
}

impl From<TextItem> for PostPayload {
	fn from(text_content: TextItem) -> Self { Self::Text(text_content) }
}

impl From<RefusalItem> for PostPayload {
	fn from(refusal_content: RefusalItem) -> Self {
		Self::Refusal(refusal_content)
	}
}

impl From<ReasoningSummaryItem> for PostPayload {
	fn from(reasoning_summary: ReasoningSummaryItem) -> Self {
		Self::ReasoningSummary(reasoning_summary)
	}
}

impl From<ReasoningContentItem> for PostPayload {
	fn from(reasoning_content: ReasoningContentItem) -> Self {
		Self::ReasoningContent(reasoning_content)
	}
}

impl From<UrlItem> for PostPayload {
	fn from(file_content: UrlItem) -> Self { Self::Url(file_content) }
}
impl From<BytesItem> for PostPayload {
	fn from(bytes_content: BytesItem) -> Self { Self::Bytes(bytes_content) }
}
impl From<FunctionCallItem> for PostPayload {
	fn from(function_call: FunctionCallItem) -> Self {
		Self::FunctionCall(function_call)
	}
}

impl From<FunctionCallOutputItem> for PostPayload {
	fn from(function_call_output: FunctionCallOutputItem) -> Self {
		Self::FunctionCallOutput(function_call_output)
	}
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum PostKind {
	Text,
	Refusal,
	ReasoningSummary,
	ReasoningContent,
	ReasoningEncryptedContent,
	Media,
	Url,
	FunctionCall,
	FunctionCallOutput,
}

impl PostKind {
	/// Whether this post is the kind that is
	/// usually presented to actors.
	pub fn is_display(&self) -> bool {
		matches!(
			self,
			Self::Text
				| Self::Refusal
				| Self::ReasoningSummary
				// | Self::ReasoningContent
				| Self::Media
				| Self::Url
		)
	}
}

/// Common type for several openresponses types
/// [`ContentPart::InputText`]
/// [`ContentPart::OutputText`] - annotations inlined as markdown
/// [`ContentPart::ReasoningSummary`]
///
/// Note that [`ContentPart::ReasoningText`] is discarded and not stored.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Serialize,
	Deserialize,
)]
pub struct TextItem(pub String);

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Serialize,
	Deserialize,
)]
pub struct RefusalItem(pub String);
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Serialize,
	Deserialize,
)]
pub struct ReasoningSummaryItem(pub String);
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Serialize,
	Deserialize,
)]
pub struct ReasoningContentItem(pub String);
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	DerefMut,
	Serialize,
	Deserialize,
)]
pub struct ReasoningEncryptedContentItem(pub String);

/// Common type for several openresponses types
/// [`ContentPart::InputImage`]
/// [`ContentPart::InputFile`]
/// [`ContentPart::InputVideo`]
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct UrlItem {
	/// The name of the file without a path or extension.
	pub file_stem: Option<String>,
	pub media_type: MediaType,
	/// The file data.
	pub url: Url,
}

impl UrlItem {
	pub fn filename(&self) -> String {
		let filename = self.file_stem.as_deref().unwrap_or_else(|| "file");
		if let Some(ext) = self.media_type.extension() {
			format!("{filename}.{}", ext)
		} else {
			filename.to_string()
		}
	}
	pub fn media_type(&self) -> &MediaType { &self.media_type }
	pub fn url(&self) -> &Url { &self.url }
}
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct BytesItem {
	/// The name of the file without a path or extension.
	pub file_stem: Option<String>,
	pub media_type: MediaType,
	/// The file data.
	pub bytes: Vec<u8>,
}

impl BytesItem {
	pub fn filename(&self) -> String {
		let filename = self.file_stem.as_deref().unwrap_or_else(|| "file");
		if let Some(ext) = self.media_type.extension() {
			format!("{filename}.{}", ext)
		} else {
			filename.to_string()
		}
	}
	pub fn media_type(&self) -> &MediaType { &self.media_type }
	pub fn bytes(&self) -> &[u8] { &self.bytes }
	pub fn bytes_base64(&self) -> String {
		base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &self.bytes)
	}
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct FunctionCallItem {
	/// The name of the function that was called, in beet this is usually
	/// the [`std::any::TypeId`] for matching against a [`ToolMeta::handler`]
	pub name: String,
	/// The arguments JSON string that was generated.
	pub arguments: String,
	/// A unique string generated by the caller for matching against the associated output
	pub call_id: String,
}

impl FunctionCallItem {
	/// The name of the function that was called.
	pub fn function_name(&self) -> &str { &self.name }
	/// The arguments JSON string.
	pub fn args(&self) -> &str { &self.arguments }
	/// A unique string generated by the caller for matching against the associated output
	pub fn call_id(&self) -> &str { &self.call_id }
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct FunctionCallOutputItem {
	/// The JSON string that was output from the tool call.
	/// Note that this should always be sent as a FunctionOutputContent::Text,
	/// regardless of if this text is json, raw text, files etc. The only purpose
	/// of sending this to models is for context, and we can provide more context through
	/// a tool specific json structure than a text, file, text etc.
	/// The only reason [`FunctionOutputContent`] is so complex is a unified type system.
	pub output: String,
	/// A unique string generated by the caller for matching against the associated output
	pub call_id: String,
}

impl FunctionCallOutputItem {
	pub fn output(&self) -> &str { &self.output }
	/// A unique string generated by the caller for matching against the associated output
	pub fn call_id(&self) -> &str { &self.call_id }
}
