use crate::prelude::*;
use beet_core::prelude::*;
use futures::Stream;
use std::borrow::Cow;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDef {
	pub provider_slug: Cow<'static, str>,
	pub model_slug: Cow<'static, str>,
	pub url: Cow<'static, str>,
	pub auth: Option<String>,
}


pub trait ActionStreamer {
	fn provider_slug(&self) -> Cow<'static, str>;
	fn model_slug(&self) -> Cow<'static, str>;

	fn stream_actions(
		&mut self,
		actor: AsyncEntity,
	) -> BoxedFuture<'_, Result<ActionStream>>;
}


pub(super) type ResPartialStream =
	Pin<Box<dyn Stream<Item = Result<ResponsePartial>> + Send + Sync>>;


/// Processes typed streaming events into [`ActionStreamOut`] values.
pub struct ActionStream {
	inner: ResPartialStream,
	agent: ActorId,
	thread: ThreadId,
	// store partial actions as they are built,
	// cloning and returning on each stream part
	actions: DocMap<Action>,
	action_partial_map: ActionPartialMap,
	provider_slug: Cow<'static, str>,
	model_slug: Cow<'static, str>,
	/// store the response partial with all actions drained,
	/// for using metadata.
	response: Option<ResponsePartial>,
}

impl ActionStream {
	pub fn new(
		provider_slug: Cow<'static, str>,
		model_slug: Cow<'static, str>,
		agent: ActorId,
		thread: ThreadId,
		inner: ResPartialStream,
	) -> Self {
		Self {
			provider_slug,
			model_slug,
			agent,
			thread,
			inner,
			actions: default(),
			action_partial_map: default(),
			response: None,
		}
	}

	/// Returns an iterator over the collected actions.
	pub fn actions(&self) -> &DocMap<Action> { &self.actions }

	pub fn meta_builder(&self) -> Result<MetaBuilder> {
		let response = self
			.response
			.as_ref()
			.ok_or_else(|| {
				bevyhow!(
					"response id is required to write actions, did the stream finish?"
				)
			})?
			.clone();
		let provider_slug = self.provider_slug.to_string();
		let model_slug = self.model_slug.to_string();
		MetaBuilder {
			provider_slug,
			model_slug,
			response_id: response.response_id.clone(),
			response_stored: response.response_stored,
		}
		.xok()
	}
}

pub struct MetaBuilder {
	provider_slug: String,
	model_slug: String,
	response_id: String,
	response_stored: bool,
}
impl MetaBuilder {
	pub fn build(&self, action_id: ActionId) -> ResponseMeta {
		ResponseMeta {
			action_id,
			provider_slug: self.provider_slug.clone(),
			model_slug: self.model_slug.clone(),
			response_id: self.response_id.clone(),
			response_stored: self.response_stored,
		}
	}
}

#[derive(Debug, Default)]
pub struct ActionChanges {
	pub created: Vec<Action>,
	pub modified: Vec<Action>,
}

impl ActionChanges {
	pub fn is_empty(&self) -> bool {
		self.created.is_empty() && self.modified.is_empty()
	}
}


impl Stream for ActionStream {
	type Item = Result<ActionChanges>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();
		match this.inner.as_mut().poll_next(cx) {
			Poll::Ready(Some(Ok(mut res_partial))) => {
				trace!("Streaming Event: {:#?}", res_partial);
				let action_partials = res_partial.take_actions();
				this.response = res_partial.xsome();

				// get next state without the actions
				let changes = this.action_partial_map.apply_actions(
					&mut this.actions,
					this.agent,
					this.thread,
					action_partials,
				)?;

				Poll::Ready(Some(Ok(changes)))
			}
			Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
