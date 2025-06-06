use crate::openai::realtime::types as models;
use serde::Deserialize;
use serde::Serialize;

/// RealtimeConversationItemWithReference : The item to add to the conversation.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeConversationItemWithReference {
	/// For an item of type (`message` | `function_call` | `function_call_output`) this field allows the client to assign the unique ID of the item. It is not required because the server will generate one if not provided.  For an item of type `item_reference`, this field is required and is a reference to any item that has previously existed in the conversation.
	#[serde(rename = "id", skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// The type of the item (`message`, `function_call`, `function_call_output`, `item_reference`).
	#[serde(rename = "type", skip_serializing_if = "Option::is_none")]
	pub r#type: Option<Type>,
	/// Identifier for the API object being returned - always `realtime.item`.
	#[serde(rename = "object", skip_serializing_if = "Option::is_none")]
	pub object: Option<Object>,
	/// The status of the item (`completed`, `incomplete`). These have no effect  on the conversation, but are accepted for consistency with the  `conversation.item.created` event.
	#[serde(rename = "status", skip_serializing_if = "Option::is_none")]
	pub status: Option<Status>,
	/// The role of the message sender (`user`, `assistant`, `system`), only  applicable for `message` items.
	#[serde(rename = "role", skip_serializing_if = "Option::is_none")]
	pub role: Option<Role>,
	/// The content of the message, applicable for `message` items.  - Message items of role `system` support only `input_text` content - Message items of role `user` support `input_text` and `input_audio`    content - Message items of role `assistant` support `text` content.
	#[serde(rename = "content", skip_serializing_if = "Option::is_none")]
	pub content: Option<Vec<models::RealtimeConversationItemContentInner>>,
	/// The ID of the function call (for `function_call` and  `function_call_output` items). If passed on a `function_call_output`  item, the server will check that a `function_call` item with the same  ID exists in the conversation history.
	#[serde(rename = "call_id", skip_serializing_if = "Option::is_none")]
	pub call_id: Option<String>,
	/// The name of the function being called (for `function_call` items).
	#[serde(rename = "name", skip_serializing_if = "Option::is_none")]
	pub name: Option<String>,
	/// The arguments of the function call (for `function_call` items).
	#[serde(rename = "arguments", skip_serializing_if = "Option::is_none")]
	pub arguments: Option<String>,
	/// The output of the function call (for `function_call_output` items).
	#[serde(rename = "output", skip_serializing_if = "Option::is_none")]
	pub output: Option<String>,
}

impl RealtimeConversationItemWithReference {
	/// The item to add to the conversation.
	pub fn new() -> RealtimeConversationItemWithReference {
		RealtimeConversationItemWithReference {
			id: None,
			r#type: None,
			object: None,
			status: None,
			role: None,
			content: None,
			call_id: None,
			name: None,
			arguments: None,
			output: None,
		}
	}
}
/// The type of the item (`message`, `function_call`, `function_call_output`, `item_reference`).
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Hash,
	Serialize,
	Deserialize,
)]
pub enum Type {
	#[serde(rename = "message")]
	Message,
	#[serde(rename = "function_call")]
	FunctionCall,
	#[serde(rename = "function_call_output")]
	FunctionCallOutput,
}

impl Default for Type {
	fn default() -> Type { Self::Message }
}
/// Identifier for the API object being returned - always `realtime.item`.
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Hash,
	Serialize,
	Deserialize,
)]
pub enum Object {
	#[serde(rename = "realtime.item")]
	RealtimePeriodItem,
}

impl Default for Object {
	fn default() -> Object { Self::RealtimePeriodItem }
}
/// The status of the item (`completed`, `incomplete`). These have no effect  on the conversation, but are accepted for consistency with the  `conversation.item.created` event.
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Hash,
	Serialize,
	Deserialize,
)]
pub enum Status {
	#[serde(rename = "completed")]
	Completed,
	#[serde(rename = "incomplete")]
	Incomplete,
}

impl Default for Status {
	fn default() -> Status { Self::Completed }
}
/// The role of the message sender (`user`, `assistant`, `system`), only  applicable for `message` items.
#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Hash,
	Serialize,
	Deserialize,
)]
pub enum Role {
	#[serde(rename = "user")]
	User,
	#[serde(rename = "assistant")]
	Assistant,
	#[serde(rename = "system")]
	System,
}

impl Default for Role {
	fn default() -> Role { Self::User }
}
