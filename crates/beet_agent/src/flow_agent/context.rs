//! Context types for flow agents.
//!
//! Context represents the conversation history and state for AI agents.
//! Each piece of context is an entity with a relationship to a flow agent,
//! allowing for flexible storage and UI display.
//!
//! # Architecture
//!
//! Context entities are linked to a flow agent via the [`ContextOf`] relationship.
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
use beet_flow::prelude::AgentQuery;
use std::path::PathBuf;


/// A piece of context belonging to a flow agent.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Context)]
pub struct ContextOf(pub Entity);


/// The Flow Agent containing an ordered collection of context.
#[derive(Deref, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ContextOf, linked_spawn)]
pub struct Context(Vec<Entity>);


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


/// Metadata for tracking context state across requests.
///
/// Each context entity tracks which action created it via `created_by`.
/// This enables proper role determination in multi-agent conversations:
/// - Context created by "me" (the current action) → Assistant role
/// - Context created by others → User role (prefixed with creator's name)
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component)]
pub struct ContextMeta {
	/// The action entity that created this context.
	/// Used to determine roles in multi-agent conversations.
	pub created_by: Option<Entity>,
	/// The response ID this context was sent with, if any.
	/// Used for `previous_response_id` support.
	pub sent_with_response_id: Option<String>,
	/// The item ID from the streaming response, if this context came from a model.
	pub item_id: Option<String>,
}

impl ContextMeta {
	/// Creates metadata with the creating action entity.
	pub fn from_action(action: Entity) -> Self {
		Self {
			created_by: Some(action),
			sent_with_response_id: None,
			item_id: None,
		}
	}

	/// Creates metadata for context that was sent in a request.
	pub fn sent(action: Entity, response_id: impl Into<String>) -> Self {
		Self {
			created_by: Some(action),
			sent_with_response_id: Some(response_id.into()),
			item_id: None,
		}
	}

	/// Creates metadata for context received from a streaming response.
	pub fn received(action: Entity, item_id: impl Into<String>) -> Self {
		Self {
			created_by: Some(action),
			sent_with_response_id: None,
			item_id: Some(item_id.into()),
		}
	}
}


/// Marks a context entity as complete (no more content will be added).
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


/// Reasoning content from the model (for reasoning models).
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component)]
pub struct ReasoningContext(pub String);

impl ReasoningContext {
	/// Creates new reasoning context.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }

	/// Appends text to this context.
	pub fn push_str(&mut self, text: &str) { self.0.push_str(text); }
}


/// File content in a context.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct FileContext {
	/// The mime type of the file.
	pub mime_type: String,
	/// The filename (for display purposes).
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
					use beet_net::prelude::RequestClientExt;
					beet_core::prelude::Request::get(url)
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


/// A view of a single context item for iteration.
#[derive(Debug)]
pub enum ContextView<'a> {
	Text(&'a TextContext),
	Reasoning(&'a ReasoningContext),
	File(&'a FileContext),
	FunctionCall(&'a FunctionCallContext),
	FunctionOutput(&'a FunctionOutputContext),
}


/// System parameter for querying context entities.
#[derive(SystemParam)]
pub struct ContextQuery<'w, 's> {
	pub contexts: AgentQuery<'w, 's, &'static Context>,
	pub text_contexts: Query<'w, 's, &'static TextContext>,
	pub reasoning_contexts: Query<'w, 's, &'static ReasoningContext>,
	pub file_contexts: Query<'w, 's, &'static FileContext>,
	pub function_calls: Query<'w, 's, &'static FunctionCallContext>,
	pub function_outputs: Query<'w, 's, &'static FunctionOutputContext>,
	pub roles: Query<'w, 's, &'static ContextRole>,
	pub metas: Query<'w, 's, &'static ContextMeta>,
	pub names: Query<'w, 's, &'static Name>,
}


impl<'w, 's> ContextQuery<'w, 's> {
	/// Get the text contexts for a given flow agent.
	pub fn texts(&self, action: Entity) -> Vec<&TextContext> {
		let mut texts = Vec::new();
		if let Ok(context) = self.contexts.get(action) {
			for ctx_entity in context.iter() {
				if let Ok(text_ctx) = self.text_contexts.get(ctx_entity) {
					texts.push(text_ctx);
				}
			}
		}
		texts
	}

	/// Get all context entities for a flow agent, filtered by whether they've been sent.
	pub fn unsent_entities(&self, action: Entity) -> Vec<Entity> {
		let mut entities = Vec::new();
		if let Ok(context) = self.contexts.get(action) {
			for ctx_entity in context.iter() {
				// Check if this context has been sent
				let is_sent = self
					.metas
					.get(ctx_entity)
					.map(|meta| meta.sent_with_response_id.is_some())
					.unwrap_or(false);
				if !is_sent {
					entities.push(ctx_entity);
				}
			}
		}
		entities
	}

	/// Convert context entities to openresponses input items.
	///
	/// This collects all context and converts it to the format expected by
	/// the OpenResponses API. Role determination in multi-agent conversations:
	/// - Context created by `action` (the current action) → Assistant role
	/// - Context created by other actions → User role, prefixed with creator's name
	///
	/// The `action` parameter identifies who "I" am in this conversation.
	pub fn to_input_items(
		&self,
		action: Entity,
	) -> Vec<openresponses::request::InputItem> {
		let mut items = Vec::new();

		if let Ok(context) = self.contexts.get(action) {
			for ctx_entity in context.iter() {
				// Determine role based on who created this context
				let meta = self.metas.get(ctx_entity).ok();
				let created_by = meta.and_then(|m| m.created_by);

				// "I created it" → Assistant, "someone else" → User
				let effective_role = if created_by == Some(action) {
					ContextRole::Assistant
				} else {
					// Use the stored role, but treat other assistants as users
					let stored_role = self
						.roles
						.get(ctx_entity)
						.copied()
						.unwrap_or(ContextRole::User);
					match stored_role {
						ContextRole::Assistant => ContextRole::User,
						other => other,
					}
				};

				// Get the creator's name for prefixing (only for non-self contexts)
				let creator_name = if created_by != Some(action) {
					created_by.and_then(|entity| {
						self.names
							.get(entity)
							.ok()
							.map(|n| n.as_str().to_string())
					})
				} else {
					None
				};

				// Build message content parts
				let mut parts = Vec::new();

				if let Ok(text) = self.text_contexts.get(ctx_entity) {
					// Prefix with creator's name if from another agent
					let text_content = if let Some(ref name) = creator_name {
						format!("{} > {}", name, text.0)
					} else {
						text.0.clone()
					};
					parts.push(openresponses::ContentPart::input_text(
						&text_content,
					));
				}

				if let Ok(file) = self.file_contexts.get(ctx_entity) {
					parts.push(file.to_content_part(
						effective_role != ContextRole::Assistant,
					));
				}

				if let Ok(reasoning) = self.reasoning_contexts.get(ctx_entity) {
					// Reasoning is typically from assistant
					let reasoning_content = if let Some(ref name) = creator_name
					{
						format!("{} > {}", name, reasoning.0)
					} else {
						reasoning.0.clone()
					};
					parts.push(openresponses::ContentPart::InputText(
						openresponses::InputText::new(&reasoning_content),
					));
				}

				// Handle function calls
				if let Ok(func_call) = self.function_calls.get(ctx_entity) {
					items
						.push(openresponses::request::InputItem::FunctionCall(
						openresponses::request::FunctionCallParam {
							id: None,
							call_id: func_call.call_id.clone(),
							name: func_call.name.clone(),
							arguments: func_call.arguments.clone(),
							status: Some(
								openresponses::FunctionCallStatus::Completed,
							),
						},
					));
					continue;
				}

				// Handle function outputs
				if let Ok(func_output) = self.function_outputs.get(ctx_entity) {
					items.push(openresponses::request::InputItem::FunctionCallOutput(
						openresponses::request::FunctionCallOutputParam::text(
							&func_output.call_id,
							&func_output.output,
						),
					));
					continue;
				}

				// Skip empty content
				if parts.is_empty() {
					continue;
				}

				// Create message with effective role
				let message =
					if parts.len() == 1 {
						if let Some(text) = parts[0].as_text() {
							match effective_role {
							ContextRole::User => {
								openresponses::request::MessageParam::user(text)
							}
							ContextRole::Assistant => {
								openresponses::request::MessageParam::assistant(text)
							}
							ContextRole::System => {
								openresponses::request::MessageParam::system(text)
							}
							ContextRole::Developer => {
								openresponses::request::MessageParam::developer(text)
							}
						}
						} else {
							openresponses::request::MessageParam {
							id: None,
							role: effective_role.to_message_role(),
							content: openresponses::request::MessageContent::Parts(parts),
							status: None,
						}
						}
					} else {
						openresponses::request::MessageParam {
							id: None,
							role: effective_role.to_message_role(),
							content:
								openresponses::request::MessageContent::Parts(
									parts,
								),
							status: None,
						}
					};

				items.push(openresponses::request::InputItem::Message(message));
			}
		}

		items
	}
}


/// Spawns context entities from streaming responses.
///
/// This handles the incremental creation and updating of context entities
/// as streaming events arrive from the model.
pub struct ContextSpawner {
	/// The async world handle for spawning entities.
	world: AsyncWorld,
	/// The flow agent entity that owns this context.
	agent: Entity,
	/// The action entity that is creating these contexts.
	action: Entity,
	/// Map from item IDs to context entities.
	item_map: HashMap<String, Entity>,
	/// Set of completed item IDs.
	completed_items: HashSet<String>,
	/// The response ID from the model.
	response_id: Option<String>,
}

impl ContextSpawner {
	/// Creates a new context spawner for the given agent and action.
	///
	/// The `action` entity is stored in `ContextMeta::created_by` for each
	/// context entity, enabling proper role determination in multi-agent conversations.
	pub fn new(world: AsyncWorld, agent: Entity, action: Entity) -> Self {
		Self {
			world,
			agent,
			action,
			item_map: HashMap::default(),
			completed_items: HashSet::default(),
			response_id: None,
		}
	}

	/// Sets the response ID (called when response.created is received).
	pub fn set_response_id(&mut self, id: impl Into<String>) {
		self.response_id = Some(id.into());
	}

	/// Gets the response ID, if set.
	pub fn response_id(&self) -> Option<&str> { self.response_id.as_deref() }

	/// Spawns a new text context entity for the given item ID.
	pub async fn spawn_text(
		&mut self,
		item_id: impl Into<String>,
	) -> Result<Entity> {
		let item_id = item_id.into();
		let entity = self
			.world
			.spawn_then((
				ContextOf(self.agent),
				TextContext::default(),
				ContextRole::Assistant,
				ContextMeta::received(self.action, &item_id),
				ContextStreaming,
			))
			.await
			.id();
		self.item_map.insert(item_id, entity);
		entity.xok()
	}

	/// Spawns a new reasoning context entity.
	pub async fn spawn_reasoning(
		&mut self,
		item_id: impl Into<String>,
	) -> Result<Entity> {
		let item_id = item_id.into();
		let entity = self
			.world
			.spawn_then((
				ContextOf(self.agent),
				ReasoningContext::default(),
				ContextRole::Assistant,
				ContextMeta::received(self.action, &item_id),
				ContextStreaming,
			))
			.await
			.id();
		self.item_map.insert(item_id, entity);
		entity.xok()
	}

	/// Spawns a new function call context entity.
	pub async fn spawn_function_call(
		&mut self,
		item_id: impl Into<String>,
		call_id: impl Into<String>,
		name: impl Into<String>,
	) -> Result<Entity> {
		let item_id = item_id.into();
		let entity = self
			.world
			.spawn_then((
				ContextOf(self.agent),
				FunctionCallContext {
					call_id: call_id.into(),
					name: name.into(),
					arguments: String::new(),
				},
				ContextRole::Assistant,
				ContextMeta::received(self.action, &item_id),
				ContextStreaming,
			))
			.await
			.id();
		self.item_map.insert(item_id, entity);
		entity.xok()
	}

	/// Gets the entity for an item ID, if it exists.
	pub fn get_entity(&self, item_id: &str) -> Option<Entity> {
		self.item_map.get(item_id).copied()
	}

	/// Appends text to an existing text context.
	pub async fn append_text(
		&mut self,
		item_id: &str,
		delta: &str,
	) -> Result<()> {
		let entity = self
			.item_map
			.get(item_id)
			.copied()
			.ok_or_else(|| bevyhow!("No entity for item ID: {item_id}"))?;

		let delta = delta.to_string();
		self.world
			.entity(entity)
			.with_then(move |mut entity| {
				if let Some(mut text) = entity.get_mut::<TextContext>() {
					text.push_str(&delta);
				} else if let Some(mut reasoning) =
					entity.get_mut::<ReasoningContext>()
				{
					reasoning.push_str(&delta);
				}
			})
			.await;
		Ok(())
	}

	/// Appends arguments to an existing function call context.
	pub async fn append_function_arguments(
		&mut self,
		item_id: &str,
		delta: &str,
	) -> Result<()> {
		let entity = self
			.item_map
			.get(item_id)
			.copied()
			.ok_or_else(|| bevyhow!("No entity for item ID: {item_id}"))?;

		let delta = delta.to_string();
		self.world
			.entity(entity)
			.with_then(move |mut entity| {
				if let Some(mut func_call) =
					entity.get_mut::<FunctionCallContext>()
				{
					func_call.arguments.push_str(&delta);
				}
			})
			.await;
		Ok(())
	}

	/// Inserts a file context for an item.
	pub async fn insert_file(
		&mut self,
		item_id: impl Into<String>,
		file: FileContext,
	) -> Result<Entity> {
		let item_id = item_id.into();

		if let Some(&entity) = self.item_map.get(&item_id) {
			// Update existing entity
			self.world.entity(entity).insert_then(file).await;
			entity.xok()
		} else {
			// Spawn new entity
			let entity = self
				.world
				.spawn_then((
					ContextOf(self.agent),
					file,
					ContextRole::Assistant,
					ContextMeta::received(self.action, &item_id),
					ContextStreaming,
				))
				.await
				.id();
			self.item_map.insert(item_id, entity);
			entity.xok()
		}
	}

	/// Marks an item as complete.
	pub async fn complete_item(&mut self, item_id: &str) -> Result<()> {
		if let Some(&entity) = self.item_map.get(item_id) {
			self.world
				.entity(entity)
				.with_then(|mut entity| {
					entity.remove::<ContextStreaming>();
					entity.insert(ContextComplete);
				})
				.await;
			self.completed_items.insert(item_id.to_string());
		}
		Ok(())
	}

	/// Completes all remaining items.
	pub async fn complete_all(&mut self) -> Result<()> {
		for (item_id, &entity) in &self.item_map {
			if !self.completed_items.contains(item_id) {
				self.world
					.entity(entity)
					.with_then(|mut entity| {
						entity.remove::<ContextStreaming>();
						entity.insert(ContextComplete);
					})
					.await;
			}
		}
		Ok(())
	}

	/// Handles a streaming event, updating context entities as needed.
	///
	/// Returns `true` if the stream is complete.
	pub async fn handle_event(
		&mut self,
		event: &openresponses::StreamingEvent,
	) -> Result<bool> {
		use openresponses::StreamingEvent::*;

		match event {
			ResponseCreated(ev) => {
				self.set_response_id(&ev.response.id);
			}
			OutputItemAdded(ev) => {
				if let Some(item) = &ev.item {
					match item {
						openresponses::OutputItem::Message(msg) => {
							self.spawn_text(&msg.id).await?;
						}
						openresponses::OutputItem::Reasoning(reasoning) => {
							self.spawn_reasoning(&reasoning.id).await?;
						}
						openresponses::OutputItem::FunctionCall(fc) => {
							self.spawn_function_call(
								&fc.id,
								&fc.call_id,
								&fc.name,
							)
							.await?;
						}
						openresponses::OutputItem::FunctionCallOutput(_) => {
							// Function call outputs are provided by us, not spawned
						}
					}
				}
			}
			OutputTextDelta(ev) => {
				self.append_text(&ev.item_id, &ev.delta).await?;
			}
			ReasoningDelta(ev) => {
				self.append_text(&ev.item_id, &ev.delta).await?;
			}
			FunctionCallArgumentsDelta(ev) => {
				self.append_function_arguments(&ev.item_id, &ev.delta)
					.await?;
			}
			OutputItemDone(ev) => {
				if let Some(item) = &ev.item {
					if let Some(id) = item.id() {
						self.complete_item(id).await?;
					}
				}
			}
			ResponseCompleted(_)
			| ResponseFailed(_)
			| ResponseIncomplete(_) => {
				self.complete_all().await?;
				return true.xok();
			}
			// Ignore other events
			_ => {}
		}

		false.xok()
	}
}


/// Marks context entities as sent with the given response ID.
pub async fn mark_contexts_sent(
	world: &AsyncWorld,
	entities: &[Entity],
	action: Entity,
	response_id: &str,
) {
	let response_id = response_id.to_string();
	for &entity in entities {
		let rid = response_id.clone();
		world
			.entity(entity)
			.with_then(move |mut entity| {
				if let Some(mut meta) = entity.get_mut::<ContextMeta>() {
					meta.sent_with_response_id = Some(rid);
				} else {
					entity.insert(ContextMeta::sent(action, rid));
				}
			})
			.await;
	}
}


/// Spawns context entities from a non-streaming response.
///
/// The `action` entity is stored in `ContextMeta::created_by` for each
/// context entity, enabling proper role determination in multi-agent conversations.
pub async fn spawn_response_context(
	world: &AsyncWorld,
	agent: Entity,
	action: Entity,
	response: &openresponses::ResponseBody,
) -> Result<Vec<Entity>> {
	let mut entities = Vec::new();

	for item in &response.output {
		match item {
			openresponses::OutputItem::Message(msg) => {
				let text = msg.all_text();
				if !text.is_empty() {
					let entity = world
						.spawn_then((
							ContextOf(agent),
							TextContext::new(text),
							ContextRole::Assistant,
							ContextMeta::received(action, &msg.id),
							ContextComplete,
						))
						.await
						.id();
					entities.push(entity);
				}
			}
			openresponses::OutputItem::Reasoning(reasoning) => {
				let text = reasoning
					.content
					.iter()
					.map(|content| content.text.as_str())
					.collect::<Vec<_>>()
					.join("");
				if !text.is_empty() {
					let entity = world
						.spawn_then((
							ContextOf(agent),
							ReasoningContext::new(text),
							ContextRole::Assistant,
							ContextMeta::received(action, &reasoning.id),
							ContextComplete,
						))
						.await
						.id();
					entities.push(entity);
				}
			}
			openresponses::OutputItem::FunctionCall(fc) => {
				let entity = world
					.spawn_then((
						ContextOf(agent),
						FunctionCallContext {
							call_id: fc.call_id.clone(),
							name: fc.name.clone(),
							arguments: fc.arguments.clone(),
						},
						ContextRole::Assistant,
						ContextMeta::received(action, &fc.id),
						ContextComplete,
					))
					.await
					.id();
				entities.push(entity);
			}
			openresponses::OutputItem::FunctionCallOutput(_) => {
				// Function outputs are typically provided by us
			}
		}
	}

	entities.xok()
}


/// Spawns a user context with the given text.
///
/// The `action` entity is stored in `ContextMeta::created_by` for tracking
/// which action created this context (typically the request_to_context action).
pub async fn spawn_user_context(
	world: &AsyncWorld,
	agent: Entity,
	action: Entity,
	text: impl Into<String>,
) -> Entity {
	world
		.spawn_then((
			ContextOf(agent),
			TextContext::new(text),
			ContextRole::User,
			ContextMeta::from_action(action),
			ContextComplete,
		))
		.await
		.id()
}


/// Spawns a system context with the given text.
///
/// The `action` entity is stored in `ContextMeta::created_by` for tracking.
pub async fn spawn_system_context(
	world: &AsyncWorld,
	agent: Entity,
	action: Entity,
	text: impl Into<String>,
) -> Entity {
	world
		.spawn_then((
			ContextOf(agent),
			TextContext::new(text),
			ContextRole::System,
			ContextMeta::from_action(action),
			ContextComplete,
		))
		.await
		.id()
}
