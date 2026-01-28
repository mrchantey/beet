use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::BodyStream;
use bevy::ecs::component::Mutable;
use std::ops::ControlFlow;

/// Spawns context entities from streaming responses.
///
/// This handles the incremental creation and updating of context entities
/// as streaming events arrive from the model.
///
/// When a [`BodyStream`] is provided, text deltas are also written to it,
/// enabling real-time streaming output to HTTP responses.
pub struct StreamingContextSpawner {
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
	/// Optional body stream for writing text deltas to HTTP responses.
	body_stream: Option<BodyStream>,
}

impl StreamingContextSpawner {
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
			body_stream: None,
		}
	}

	/// Sets the body stream for writing text deltas to HTTP responses.
	///
	/// When set, text deltas will be written to this stream in addition to
	/// updating the context entities.
	pub fn with_body_stream(mut self, body_stream: BodyStream) -> Self {
		self.body_stream = Some(body_stream);
		self
	}

	/// Handles a streaming event, updating context entities as needed.
	///
	/// Returns `true` if the stream is complete.
	pub async fn handle_event(
		&mut self,
		event: &openresponses::StreamingEvent,
	) -> Result<ControlFlow<()>> {
		use openresponses::StreamingEvent::*;
		info!("Handling streaming event: {:#?}", event);
		match event {
			ResponseCreated(ev) => {
				self.spawn_response(&ev.response).await?;
			}
			OutputItemAdded(ev) => {
				if let Some(item) = &ev.item {
					self.spawn_output_item(item).await?;
				}
			}
			OutputTextDelta(ev) => {
				self.update_text(ev).await?;
			}
			ReasoningDelta(ev) => {
				self.update_resoning(&ev).await?;
			}
			FunctionCallArgumentsDelta(ev) => {
				self.update_function_call(&ev).await?;
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
				return ControlFlow::Break(()).xok();
			}
			// Ignore other events
			ev => {
				trace!("Unhandled streaming event: {:?}", ev);
			}
		}

		ControlFlow::Continue(()).xok()
	}
	/// Gets the response ID, if set.
	pub fn response_id(&self) -> Option<&str> { self.response_id.as_deref() }

	/// Sets the response ID (called when response.created is received).
	async fn spawn_response(
		&mut self,
		response: &openresponses::ResponseBody,
	) -> Result {
		self.response_id = Some(response.id.clone());
		for item in &response.output {
			self.spawn_output_item(item).await?;
		}
		Ok(())
	}

	async fn spawn_output_item(
		&mut self,
		item: &openresponses::OutputItem,
	) -> Result<Entity> {
		match item {
			openresponses::OutputItem::Message(msg) => {
				self.spawn_item(&msg.id, TextContext::from(msg)).await
			}
			openresponses::OutputItem::Reasoning(reasoning) => {
				self.spawn_item(
					&reasoning.id,
					ReasoningContext::from(reasoning),
				)
				.await
			}
			openresponses::OutputItem::FunctionCall(fc) => {
				self.spawn_item(&fc.id, FunctionCallContext::from(fc)).await
			}
			openresponses::OutputItem::FunctionCallOutput(fco) => {
				self.spawn_item(&fco.id, FunctionOutputContext::from(fco))
					.await
			}
		}
	}

	async fn spawn_item(
		&mut self,
		item_id: &str,
		item: impl Bundle,
	) -> Result<Entity> {
		let entity = self
			.world
			.spawn_then((
				ThreadContextOf(self.agent),
				OwnedContextOf(self.action),
				ContextRole::Assistant,
				ContextStreaming,
				item,
			))
			.await
			.id();
		self.item_map.insert(item_id.to_string(), entity);
		entity.xok()
	}

	async fn update_item<C: Component<Mutability = Mutable>>(
		&mut self,
		item_id: &str,
		func: impl 'static + Send + FnOnce(Mut<'_, C>) -> Result,
	) -> Result {
		let item_id = item_id.to_string();
		let entity_id = self
			.item_map
			.get(&item_id)
			.copied()
			.ok_or_else(|| bevyhow!("No entity for item ID: {item_id}"))?;

		self.world
			.entity(entity_id)
			.with_then(move |mut entity| {
				if let Some(comp) = entity.get_mut::<C>() {
					func(comp)?;
					Ok(())
				} else {
					bevybail!(
						"Entity: {} for item ID: {} is missing component {}",
						entity.id(),
						item_id,
						std::any::type_name::<C>()
					)
				}
			})
			.await?;
		Ok(())
	}


	/// Appends text to an existing text context.
	///
	/// Also writes to the body stream if one is configured.
	async fn update_text(
		&mut self,
		ev: &openresponses::streaming::OutputTextDeltaEvent,
	) -> Result {
		let delta = ev.delta.to_string();

		// Write to body stream if available
		if let Some(stream) = &self.body_stream {
			stream.send_text(&delta).await?;
		}

		self.update_item::<TextContext>(&ev.item_id, move |mut item| {
			item.push_str(&delta);
			Ok(())
		})
		.await
	}
	/// Appends text to an existing text context.
	async fn update_resoning(
		&mut self,
		ev: &openresponses::streaming::ReasoningDeltaEvent,
	) -> Result {
		let delta = ev.delta.to_string();
		self.update_item::<ReasoningContext>(&ev.item_id, move |mut item| {
			item.push_str(&delta);
			Ok(())
		})
		.await
	}

	/// Appends arguments to an existing function call context.
	async fn update_function_call(
		&mut self,
		ev: &openresponses::streaming::FunctionCallArgumentsDeltaEvent,
	) -> Result {
		let delta = ev.delta.to_string();
		self.update_item::<FunctionCallContext>(&ev.item_id, move |mut item| {
			item.arguments.push_str(&delta);
			Ok(())
		})
		.await
	}

	/// Marks an item as complete.
	async fn complete_item(&mut self, item_id: &str) -> Result {
		if let Some(&entity) = self.item_map.get(item_id) {
			self.world
				.entity(entity)
				.with_then(|mut entity| {
					entity.remove::<ContextStreaming>().insert(ContextComplete);
				})
				.await;
			self.completed_items.insert(item_id.to_string());
		}
		Ok(())
	}

	/// Completes all remaining items.
	async fn complete_all(&mut self) -> Result {
		for (item_id, &entity) in &self.item_map {
			if !self.completed_items.contains(item_id) {
				self.world
					.entity(entity)
					.with_then(|mut entity| {
						entity
							.remove::<ContextStreaming>()
							.insert(ContextComplete);
					})
					.await;
			}
		}
		Ok(())
	}
}
