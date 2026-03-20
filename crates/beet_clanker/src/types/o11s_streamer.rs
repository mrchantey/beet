#![allow(unused, reason = "todo")]
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use std::pin::Pin;



#[derive(Debug)]
pub struct O11sStreamer {
	/// The model name to use for requests.
	model: String,
	/// Whether to use streaming mode.
	stream: bool,
	/// System instructions to include with each request.
	instructions: Option<String>,
	/// Providers may track the last sent id
	previous_response_id: Option<String>,
	/// Track which action was sent last, for skipping sent
	/// actions when a previous_response_id is used.
	last_action_sent: Option<ActionId>,
	partial_items: PartialItemMap,
	auth: Option<String>,
	url: String,
}

impl O11sStreamer {
	pub fn new(url: impl Into<String>, model: impl Into<String>) -> Self {
		Self {
			model: model.into(),
			url: url.into(),
			stream: true,
			instructions: None,
			previous_response_id: None,
			last_action_sent: None,
			partial_items: default(),
			auth: None,
		}
	}
	pub fn with_auth(mut self, auth: impl Into<String>) -> Self {
		self.auth = Some(auth.into());
		self
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
	fn build_request(
		&self,
		request: openresponses::RequestBody,
	) -> Result<Request> {
		let mut request = Request::post(&self.url)
			.with_json_body::<openresponses::RequestBody>(&request)?;
		if let Some(auth) = &self.auth {
			request = request.with_auth_bearer(auth);
		}
		request.xok()
	}
}


impl ActionStreamer for O11sStreamer {
	fn last_action_sent(&self) -> Option<ActionId> { self.last_action_sent }
	fn stream_actions(
		&mut self,
		_actor: ActorId,
		_thread: ThreadId,
		_context_map: &ContextMap,
	) -> BoxedFuture<'_, Result<ActionStream>> {
		Box::pin(async move { bevybail!("todo") })
	}
}
