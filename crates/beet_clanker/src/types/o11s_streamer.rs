use crate::openresponses::request::Input;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use std::borrow::Cow;
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::Arc;

pub struct O11sStreamer {
	action_store: Arc<dyn ActionStore>,
	model: ModelDef,
	/// Whether to use streaming mode.
	stream: bool,
	/// whether to find the previous response if it exists in the thread,
	/// and attempt to pick up where it left off. This should be disabled
	/// for providers who ignore it or are stateless between calls, like ollama
	use_previous_response_id: bool,
	/// System instructions to include with each request.
	instructions: Option<String>,
}

impl std::fmt::Debug for O11sStreamer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("O11sStreamer")
			.field("model", &self.model)
			.field("stream", &self.stream)
			.field("use_previous_response_id", &self.use_previous_response_id)
			.field("instructions", &self.instructions)
			.finish()
	}
}

impl O11sStreamer {
	pub fn new(store: impl 'static + ActionStore, model: ModelDef) -> Self {
		Self {
			action_store: Arc::new(store),
			model,
			stream: true,
			use_previous_response_id: false,
			instructions: None,
		}
	}

	/// Enables streaming mode.
	pub fn without_streaming(mut self) -> Self {
		self.stream = false;
		self
	}
	/// Sets the instructions for this model action.
	pub fn with_instructions(
		mut self,
		instructions: impl Into<String>,
	) -> Self {
		self.instructions = Some(instructions.into());
		self
	}
	pub fn with_use_previous_response_id(mut self) -> Self {
		self.use_previous_response_id = true;
		self
	}

	fn build_request(
		&self,
		request: openresponses::RequestBody,
	) -> Result<Request> {
		let mut request =
			Request::post(&self.model.url)
				.with_json_body::<openresponses::RequestBody>(&request)?;
		if let Some(auth) = &self.model.auth {
			request = request.with_auth_bearer(auth);
		}
		request.xok()
	}
}


impl ActionStreamer for O11sStreamer {
	fn stream_actions(
		&mut self,
		agent_id: ActorId,
		thread_id: ThreadId,
	) -> BoxedFuture<'_, Result<ActionStream>> {
		Box::pin(async move {
			let last_received = if self.use_previous_response_id {
				self.action_store
					.previous_o11s_meta(
						&self.model.provider_slug,
						&self.model.model_slug,
						thread_id,
					)
					.await?
			} else {
				None
			};

			let items = self
				.action_store
				.full_thread_actions(thread_id, last_received)
				.await?
				.into_iter()
				.xtry_map(|(action, author, meta)| {
					o11s_mapper::action_to_o11s_input(
						agent_id, action, author, meta,
					)
				})?;

			let tools = vec![];
			let mut body = openresponses::RequestBody::new(&self.model.url)
				.with_input(Input::Items(items))
				.with_tools(tools);
			bevybail!("todo")
		})
	}
}
