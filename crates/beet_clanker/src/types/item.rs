use std::borrow::Cow;

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Url;
use serde::Deserialize;
use serde::Serialize;

pub type ItemId = DocId<Item>;

/// Note that MessageRole is not stored
/// as this is relative to the Actor.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Item {
	id: ItemId,
	/// The actor that created this item, used for attribution and scoping.
	owner: ActorId,
	status: ItemStatus,
	/// For function calls this is the time the call was completed.
	created: Timestamp,
	content: Content,
}

impl Document for Item {
	fn id(&self) -> DocId<Self> { self.id }
}


impl Item {
	pub fn new(
		owner: ActorId,
		status: ItemStatus,
		content: impl Into<Content>,
	) -> Self {
		Self {
			id: DocId::new_now(),
			owner,
			status,
			content: content.into(),
			created: Timestamp::now(),
		}
	}
	pub fn owner(&self) -> ActorId { self.owner }
	pub fn created(&self) -> Timestamp { self.created }
	pub fn content(&self) -> &Content { &self.content }
	pub(super) fn set_status(&mut self, status: ItemStatus) {
		self.status = status;
	}
	pub fn hash(&self) -> u64 {
		use std::hash::Hash;
		use std::hash::Hasher;
		let mut hasher = std::collections::hash_map::DefaultHasher::new();
		self.content.hash(&mut hasher);
		hasher.finish()
	}
	pub(super) fn content_mut(&mut self) -> &mut Content { &mut self.content }
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
pub enum ItemStatus {
	Completed,
	Interrupted,
	InProgress,
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Content {
	Text(TextItem),
	Refusal(RefusalItem),
	ReasoningSummary(ReasoningSummaryItem),
	ReasoningContent(ReasoningContentItem),
	Url(UrlItem),
	Bytes(BytesItem),
	FunctionCall(FunctionCallItem),
	FunctionCallOutput(FunctionCallOutputItem),
}

impl Content {
	pub fn kind(&self) -> ItemKind {
		match self {
			Self::Text(_) => ItemKind::Text,
			Self::Refusal(_) => ItemKind::Refusal,
			Self::ReasoningSummary(_) => ItemKind::ReasoningSummary,
			Self::ReasoningContent(_) => ItemKind::ReasoningContent,
			Self::Url(_) => ItemKind::Url,
			Self::Bytes(_) => ItemKind::Media,
			Self::FunctionCall(_) => ItemKind::FunctionCall,
			Self::FunctionCallOutput(_) => ItemKind::FunctionCallOutput,
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
				"FunctionCallItem: name={}, arguments={}",
				function_call.function_name(),
				function_call.args()
			),
			Self::FunctionCallOutput(function_call_output) => format!(
				"FunctionCallOutputItem: function_call_item={}, output={}",
				function_call_output.function_call_item,
				function_call_output.output()
			),
		}
	}
}

impl std::fmt::Display for Content {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.to_string())
	}
}

impl From<&str> for Content {
	fn from(text_content: &str) -> Self {
		Self::Text(TextItem(text_content.to_string()))
	}
}
impl From<String> for Content {
	fn from(text_content: String) -> Self { Self::Text(TextItem(text_content)) }
}
impl<'a> From<Cow<'a, String>> for Content {
	fn from(text_content: Cow<'a, String>) -> Self {
		Self::Text(TextItem(text_content.to_string()))
	}
}

impl From<TextItem> for Content {
	fn from(text_content: TextItem) -> Self { Self::Text(text_content) }
}

impl From<RefusalItem> for Content {
	fn from(refusal_content: RefusalItem) -> Self {
		Self::Refusal(refusal_content)
	}
}

impl From<ReasoningSummaryItem> for Content {
	fn from(reasoning_summary: ReasoningSummaryItem) -> Self {
		Self::ReasoningSummary(reasoning_summary)
	}
}

impl From<ReasoningContentItem> for Content {
	fn from(reasoning_content: ReasoningContentItem) -> Self {
		Self::ReasoningContent(reasoning_content)
	}
}

impl From<UrlItem> for Content {
	fn from(file_content: UrlItem) -> Self { Self::Url(file_content) }
}
impl From<BytesItem> for Content {
	fn from(bytes_content: BytesItem) -> Self { Self::Bytes(bytes_content) }
}
impl From<FunctionCallItem> for Content {
	fn from(function_call: FunctionCallItem) -> Self {
		Self::FunctionCall(function_call)
	}
}

impl From<FunctionCallOutputItem> for Content {
	fn from(function_call_output: FunctionCallOutputItem) -> Self {
		Self::FunctionCallOutput(function_call_output)
	}
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum ItemKind {
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

impl ItemKind {
	pub fn non_display_kinds() -> Vec<Self> {
		vec![
			// TODO ollama swap reasoning and content
			Self::ReasoningSummary,
			Self::ReasoningContent,
			Self::ReasoningEncryptedContent,
			Self::FunctionCall,
			Self::FunctionCallOutput,
		]
	}
	pub fn display_kinds() -> Vec<Self> {
		vec![Self::ReasoningContent, Self::Media, Self::Url]
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
}

impl FunctionCallItem {
	/// The name of the function that was called.
	pub fn function_name(&self) -> &str { &self.name }
	/// The arguments JSON string.
	pub fn args(&self) -> &str { &self.arguments }
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct FunctionCallOutputItem {
	pub function_call_item: ItemId,
	/// The JSON string that was output from the tool call.
	/// Note that this should always be sent as a FunctionOutputContent::Text,
	/// regardless of if this text is json, raw text, files etc. The only purpose
	/// of sending this to models is for context, and we can provide more context through
	/// a tool specific json structure than a text, file, text etc.
	/// The only reason [`FunctionOutputContent`] is so complex is a unified type system.
	pub output: String,
}

impl FunctionCallOutputItem {
	pub fn function_call_item(&self) -> ItemId { self.function_call_item }
	pub fn output(&self) -> &str { &self.output }
}
