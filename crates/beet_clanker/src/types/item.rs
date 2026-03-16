use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::Url;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

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
#[component(on_add=on_add)]
pub struct Item {
	id: ItemId,
	/// The actor that created this item, used for attribution and scoping.
	owner: ActorId,
	/// For function calls this is the time the call was completed.
	created: Timestamp,
	scope: ItemScope,
	content: Content,
}


fn on_add(mut world: DeferredWorld, cx: HookContext) {
	let item_id = world.entity(cx.entity).get::<Item>().unwrap().id;
	world
		.resource_mut::<ContextMap>()
		.add_item(item_id, cx.entity);
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
	/// The item is scoped only to this actor and its context,
	/// it should generally not be merged by other contexts.
	#[default]
	Actor,
	/// The item is added only to a specific list of actors,
	/// which may or may not include its owner
	Actors(Vec<ActorId>),
	/// The item is accessible to all actors in the world.
	World,
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

/// The kind of actor this entity is.
/// Distinct from [`OpenResponses::MessageRole`] in that
/// [`ActorKind`] is absolute, whereas the difference between
/// `MessageRole::User` and `MessageRole::Asssistant` is relative
/// to the agent
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
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
	/// Messages generated by this actor will always be treated as [`MessageRole::System`]
	System,
	/// Messages generated by this actor will always be treated as [`MessageRole::Developer`]
	Developer,
	/// Messages generated by this actor will always be treated as [`MessageRole::User`]
	Human,
	/// Messages generated by this actor may be treated as [`MessageRole::Assistant`] or [`MessageRole::User`],
	/// depending on the originator of the message.
	Agent,
}


/// A partial content item, which may be created from various sources and
/// may be merged into a full content item if it fully resolves.
pub enum PartialContent {
	FunctionCall {
		/// The unique ID of the function tool call, shared
		/// by both this item and its associated [`FunctionCall`]
		call_id: String,
		/// The name of the function that was called, in beet this is usually
		/// the [`std::any::TypeId`] for matching against a [`ToolMeta::handler`]
		name: String,
		/// The arguments JSON string that was generated.
		arguments: String,
	},
	/// Created by an `OutputItem::Message(Message::content)`
	Text(String),
	/// Created by an `OutputItem::Reasoning(ReasoningItem::summary)`
	ReasoningDescription(Vec<String>),
	/// The actual reasoning
	/// Created by an `OutputItem::Reasoning(ReasoningItem::content)`,
	/// this is often not shown to users by default, but could be enabled by a setting
	ReasoningContent(Vec<String>),
}
