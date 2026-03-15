//!
//!```
//!Entity Structure
//!
//! Context
//!   Actor::human
//!   Actor::human
//!   Tools, Actor::agent
//!   Tools, Actor::agent
//!```
use crate::openresponses::MessageRole;
use beet_core::prelude::*;
use beet_net::prelude::Url;
use serde::Deserialize;
use serde::Serialize;

#[derive(Component)]
pub struct Context {
	/// An append-only list of content created by various actors.
	items: Vec<Item>,
	actor_id_increment: u64,
	function_call_id_increment: u64,
}

impl Context {
	pub fn new() -> Self {
		Self {
			items: Vec::new(),
			actor_id_increment: 0,
			function_call_id_increment: 0,
		}
	}
	pub fn items(&self) -> &[Item] { &self.items }
	pub fn new_actor(&mut self, kind: ActorKind) -> Actor {
		let id = self.actor_id_increment;
		self.actor_id_increment += 1;
		Actor {
			kind,
			id: ActorId(id),
		}
	}
	pub fn new_function_call_id(&mut self) -> FunctionCallId {
		let id = self.function_call_id_increment;
		self.function_call_id_increment += 1;
		FunctionCallId(id)
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


/// Note that MessageRole is not stored
/// as this is relative to the Actor.
pub struct Item {
	actor_id: ActorId,
	created: Timestamp,
	content: Content,
}

impl Item {
	pub fn new(actor_id: ActorId, content: Content) -> Self {
		Self {
			actor_id,
			created: Timestamp::now(),
			content,
		}
	}
	pub fn actor_id(&self) -> ActorId { self.actor_id }
	pub fn created(&self) -> Timestamp { self.created }
	pub fn content(&self) -> &Content { &self.content }
}
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Content {
	Text(TextContent),
	File(FileContent),
	FunctionCall(FunctionCall),
	FunctionCallOutput(FunctionCallOutput),
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
	pub file_data: Url,
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct FunctionCall {
	/// The unique ID of the function tool call, shared
	/// by both this item and its associated [`FunctionCallOutput`]
	pub call_id: FunctionCallId,
	/// The name of the function that was called.
	pub name: String,
	/// The arguments JSON string that was generated.
	pub arguments: String,
}

#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct FunctionCallOutput {
	/// The unique ID of the function tool call, shared
	/// by both this item and its associated [`FunctionCall`]
	pub call_id: FunctionCallId,
	/// The name of the function that was called.
	pub name: String,
	/// The JSON string that was output from the tool call.
	/// Note that this should always be sent as a FunctionOutputContent::Text,
	/// regardless of if this text is json, raw text, files etc. The only purpose
	/// of sending this to models is for context, and we can provide more context through
	/// a tool specific json structure than a vec of text, file, text etc.
	/// The only reason [`FunctionOutputContent`] is so complex is a unified type system.
	pub output: String,
}


pub enum Message {}

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
pub struct Actor {
	kind: ActorKind,
	id: ActorId,
}

impl Actor {
	pub fn message_role(&self, agent_id: ActorId) -> MessageRole {
		match self.kind {
			ActorKind::System => MessageRole::System,
			ActorKind::Developer => MessageRole::Developer,
			ActorKind::Human => MessageRole::User,
			ActorKind::Agent => {
				if self.id == agent_id {
					MessageRole::Assistant
				} else {
					MessageRole::User
				}
			}
		}
	}
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


#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Deref,
	Ord,
	Hash,
	Serialize,
	Deserialize,
)]
pub struct ActorId(u64);

impl ActorId {
	pub fn new(id: u64) -> Self { Self(id) }
	pub fn inner(&self) -> u64 { self.0 }
}

/// A unique identifier for a function call, shared between the [`FunctionCall`] and its associated [`FunctionCallOutput`]
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Deref,
	PartialOrd,
	Ord,
	Hash,
	Serialize,
	Deserialize,
)]
pub struct FunctionCallId(u64);

impl FunctionCallId {
	pub fn new(id: u64) -> Self { Self(id) }
	pub fn inner(&self) -> u64 { self.0 }
}


pub enum PartialContext {
	/// Created by an `OutputItem::Message(Message::content)`
	MessageContent(String),
	/// Created by an `OutputItem::Reasoning(ReasoningItem::summary)`
	ReasoningDescription(Vec<String>),
	/// The actual reasoning
	/// Created by an `OutputItem::Reasoning(ReasoningItem::content)`,
	/// this is often not shown to users by default, but could be enabled by a setting
	ReasoningContent(Vec<String>),
}
