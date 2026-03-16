use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Url;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Note that MessageRole is not stored
/// as this is relative to the Actor.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Item {
	id: ItemId,
	/// The actor that created this item, used for attribution and scoping.
	owner: ActorId,
	/// For function calls this is the time the call was completed.
	created: Timestamp,
	scope: ItemScope,
	content: Content,
}

impl Item {
	pub fn new(actor_id: ActorId, content: Content, scope: ItemScope) -> Self {
		Self {
			id: ItemId::default(),
			owner: actor_id,
			scope,
			content,
			created: Timestamp::now(),
		}
	}
	pub fn id(&self) -> ItemId { self.id }
	pub fn owner(&self) -> ActorId { self.owner }
	pub fn created(&self) -> Timestamp { self.created }
	pub fn content(&self) -> &Content { &self.content }
	pub fn scope(&self) -> &ItemScope { &self.scope }
}

/// Id associated with an [`Item`].
/// Items are components and already have an associated [`Entity`],
/// but we need something more easily handled by distributed systems,
/// databases etc. See also [`ActorId`]
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
pub struct ItemId(Uuid);

impl std::fmt::Display for ItemId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl Default for ItemId {
	fn default() -> Self { Self(Uuid::now_v7()) }
}

#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
)]
pub enum ItemScope {
	/// The item is scoped only to this actor,
	/// and should not be added to the items of other actors.
	#[default]
	Actor,
	/// The item is accessible to only a specific list of actors,
	/// possibly exclusive of its owner, ie System items.
	ActorList(Vec<ActorId>),
	/// The item is accessible to all descendants from this
	/// actors root.
	Family,
	/// The item is accessible to all actors in the world.
	World,
}

impl ItemScope {
	pub fn single_actor(actor_id: ActorId) -> Self {
		Self::ActorList(vec![actor_id])
	}
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
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Content {
	Text(TextContent),
	File(FileContent),
	FunctionCall(FunctionCall),
}

impl Content {
	pub fn text(text: impl Into<String>) -> Self {
		Self::Text(TextContent {
			content: text.into(),
		})
	}
}

/// Common type for several openresponses types
/// [`ContentPart::InputText`]
/// [`ContentPart::OutputText`] - annotations inlined as markdown
/// [`ContentPart::ReasoningSummary`]
///
/// Note that [`ContentPart::ReasoningText`] is discarded and not stored.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct TextContent {
	content: String,
}
impl TextContent {
	pub fn content(&self) -> &str { &self.content }
}

/// Common type for several openresponses types
/// [`ContentPart::InputImage`]
/// [`ContentPart::InputFile`]
/// [`ContentPart::InputVideo`]
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct FileContent {
	/// The name of the file without a path or extension.
	pub file_stem: Option<String>,
	pub media_type: MediaType,
	/// The file data.
	pub url: Url,
}

impl FileContent {
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
pub struct FunctionCall {
	/// The name of the function that was called, in beet this is usually
	/// the [`std::any::TypeId`] for matching against a [`ToolMeta::handler`]
	pub name: String,
	/// The arguments JSON string that was generated.
	pub arguments: String,
	/// The JSON string that was output from the tool call.
	/// Note that this should always be sent as a FunctionOutputContent::Text,
	/// regardless of if this text is json, raw text, files etc. The only purpose
	/// of sending this to models is for context, and we can provide more context through
	/// a tool specific json structure than a text, file, text etc.
	/// The only reason [`FunctionOutputContent`] is so complex is a unified type system.
	pub output: String,
}

impl FunctionCall {
	/// The name of the function that was called.
	pub fn function_name(&self) -> &str { &self.name }
	/// The arguments JSON string.
	pub fn args(&self) -> &str { &self.arguments }
	/// The output JSON string.
	pub fn output(&self) -> &str { &self.output }
}
