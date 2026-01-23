//! Context types for flow agents.
//!
//! Context represents the conversation history and state for AI agents.
//! Each piece of context is an entity with a relationship to a flow agent,
//! allowing for flexible storage and UI display.
//!
//! # Architecture
//!
//! Context entities are linked to a flow agent via the [`ThreadContextOf`] relationship.
//! Each context entity has:
//! - A content component ([`TextContext`], [`FileContext`], etc.)
//! - A role component ([`ContextRole`]) indicating who created it
//! - Optional metadata ([`ContextMeta`]) for tracking streaming state
//!
//! # Converting to OpenResponses
//!
//! Use [`ContextQuery::to_input_items`] to convert context entities into
//! `openresponses::InputItem` for sending to a model.
//!
//! # Receiving from OpenResponses
//!
//! Use [`ContextSpawner`] to handle streaming responses and spawn context
//! entities incrementally.

use crate::prelude::*;
use base64::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::path::PathBuf;


/// Points to the Flow agent running this thread
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = ThreadContext)]
pub struct ThreadContextOf(pub Entity);


/// An ordered collection of context this Flow agent is responsible for
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ThreadContextOf, linked_spawn)]
pub struct ThreadContext(Vec<Entity>);

/// Points to the agent, usually an action, who created this context.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = OwnedContext)]
pub struct OwnedContextOf(pub Entity);


/// Collection of context that this agent, usually an action, created.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = OwnedContextOf, linked_spawn)]
pub struct OwnedContext(Vec<Entity>);



/// The role of the entity that created this context.
/// The user and assistant roles will not nessecarily be reflected
/// in requests to support model-model interactions.
/// ie to a model any context item created by 'me' will
/// have Assistant role and any other Assistant messages will be
/// converted to User, to give the model a clear understanding of what
/// it created.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub enum ContextRole {
	/// Context from the user.
	#[default]
	User,
	/// Context from the AI assistant.
	Assistant,
	/// Context from the system/developer.
	System,
	/// Context from developer instructions.
	Developer,
}

impl ContextRole {
	/// Converts to the openresponses message role.
	pub fn to_message_role(&self) -> openresponses::MessageRole {
		match self {
			Self::User => openresponses::MessageRole::User,
			Self::Assistant => openresponses::MessageRole::Assistant,
			Self::System => openresponses::MessageRole::System,
			Self::Developer => openresponses::MessageRole::Developer,
		}
	}
}


/// Marks a context entity as complete and immutable (no more content will be added).
#[derive(Debug, Clone, Copy, Default, Component, Reflect)]
#[reflect(Component)]
pub struct ContextComplete;

/// Marks a context entity as currently streaming.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component)]
pub struct ContextStreaming;


/// Text content in a context.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component)]
pub struct TextContext(pub String);

impl TextContext {
	/// Creates a new text context.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }

	/// Appends text to this context.
	pub fn push_str(&mut self, text: &str) { self.0.push_str(text); }
}

impl std::fmt::Display for TextContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<&openresponses::Message> for TextContext {
	fn from(item: &openresponses::Message) -> Self { Self(item.all_text()) }
}

/// Reasoning content from the model (for reasoning models),
/// This will either be the content or the summary of the reasoning,
/// summary preferred.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component)]
pub struct ReasoningContext(pub String);

impl ReasoningContext {
	/// Creates new reasoning context.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }

	/// Appends text to this context.
	pub fn push_str(&mut self, text: &str) { self.0.push_str(text); }
}
impl From<&openresponses::ReasoningItem> for ReasoningContext {
	fn from(item: &openresponses::ReasoningItem) -> Self {
		Self(item.summary_or_text())
	}
}

/// Function call context (when the model requests a tool call).
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct FunctionCallContext {
	/// The unique ID of the function call.
	pub call_id: String,
	/// The name of the function to call.
	pub name: String,
	/// The JSON arguments string.
	pub arguments: String,
}

impl From<&openresponses::FunctionCall> for FunctionCallContext {
	fn from(fc: &openresponses::FunctionCall) -> Self {
		Self {
			call_id: fc.call_id.clone(),
			name: fc.name.clone(),
			arguments: fc.arguments.clone(),
		}
	}
}

impl FunctionCallContext {
	/// Parses the arguments as a typed value.
	pub fn parse_arguments<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
		serde_json::from_str(&self.arguments).map_err(|err| {
			bevyhow!("Failed to parse function arguments: {err}")
		})
	}
}


/// Function call output context (the result of a tool call).
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct FunctionOutputContext {
	/// The call ID this output is for.
	pub call_id: String,
	/// The output of the function.
	pub output: String,
}


impl From<&openresponses::FunctionCallOutputItem> for FunctionOutputContext {
	fn from(fo: &openresponses::FunctionCallOutputItem) -> Self {
		Self {
			call_id: fo.call_id.clone(),
			output: fo.output.clone(),
		}
	}
}

/// File content in a context.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct FileContext {
	/// The mime type of the file.
	pub mime_type: String,
	/// The filename (for display purposes).
	/// May or may not be on this filesystem
	pub filename: PathBuf,
	/// The file data.
	pub data: FileContextData,
}

impl FileContext {
	/// Creates file context from a URL.
	pub fn from_url(
		url: impl Into<String>,
		filename: impl Into<PathBuf>,
	) -> Self {
		let filename = filename.into();
		let mime_type = mime_guess::from_path(&filename)
			.first_or_octet_stream()
			.essence_str()
			.to_string();
		Self {
			mime_type,
			filename,
			data: FileContextData::Url(url.into()),
		}
	}

	/// Creates file context from base64 data.
	pub fn from_base64(
		base64: impl Into<String>,
		filename: impl Into<PathBuf>,
	) -> Self {
		let filename = filename.into();
		let mime_type = mime_guess::from_path(&filename)
			.first_or_octet_stream()
			.essence_str()
			.to_string();
		Self {
			mime_type,
			filename,
			data: FileContextData::Base64(base64.into()),
		}
	}

	/// Creates file context from base64 data with explicit extension.
	pub fn from_base64_ext(
		base64: impl Into<String>,
		file_stem: &str,
		ext: &str,
	) -> Self {
		let mime_type = mime_guess::from_ext(ext)
			.first_or_octet_stream()
			.essence_str()
			.to_string();
		let filename = format!("{}.{}", file_stem, ext).into();
		Self {
			mime_type,
			filename,
			data: FileContextData::Base64(base64.into()),
		}
	}

	/// Returns whether this is an image file.
	pub fn is_image(&self) -> bool { self.mime_type.starts_with("image/") }

	/// Returns the file extension.
	pub fn extension(&self) -> Option<&str> {
		self.filename.extension().and_then(|ext| ext.to_str())
	}

	/// Converts to a data URL for embedding.
	pub fn to_data_url(&self) -> String {
		match &self.data {
			FileContextData::Base64(base64) => {
				format!("data:{};base64,{}", self.mime_type, base64)
			}
			FileContextData::Url(url) => url.clone(),
		}
	}

	/// Converts to an openresponses content part.
	pub fn to_content_part(
		&self,
		is_input: bool,
	) -> openresponses::ContentPart {
		if self.is_image() {
			if is_input {
				openresponses::ContentPart::InputImage(
					openresponses::InputImage::from_url(self.to_data_url()),
				)
			} else {
				// Output images are still represented as input_image in OpenResponses
				openresponses::ContentPart::InputImage(
					openresponses::InputImage::from_url(self.to_data_url()),
				)
			}
		} else {
			openresponses::ContentPart::InputFile(match &self.data {
				FileContextData::Url(url) => {
					openresponses::InputFile::from_url(url)
				}
				FileContextData::Base64(base64) => {
					openresponses::InputFile::from_base64(format!(
						"data:{};base64,{}",
						self.mime_type, base64
					))
				}
			})
		}
	}
}


/// The underlying data for a file context.
#[derive(Debug, Clone, Reflect)]
pub enum FileContextData {
	/// A URL (http/https or data URL).
	Url(String),
	/// Base64-encoded binary data.
	Base64(String),
}

impl FileContextData {
	/// Returns the raw bytes of the file data.
	pub async fn get_bytes(&self) -> Result<Vec<u8>> {
		match self {
			Self::Base64(base64) => BASE64_STANDARD.decode(base64)?.xok(),
			Self::Url(url) => {
				if url.starts_with("data:") {
					// Parse data URL
					let parts: Vec<&str> = url.splitn(2, ",").collect();
					if parts.len() != 2 {
						bevybail!("Invalid data URL: {}", url);
					}
					if parts[0].ends_with(";base64") {
						BASE64_STANDARD.decode(parts[1])?.xok()
					} else {
						bevybail!(
							"Only base64-encoded data URLs are supported: {}",
							url
						);
					}
				} else {
					// Fetch from URL
					Request::get(url)
						.send()
						.await?
						.into_result()
						.await?
						.bytes()
						.await
						.map(|bytes| bytes.to_vec())
				}
			}
		}
	}
}
