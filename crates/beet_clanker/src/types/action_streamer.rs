use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use std::borrow::Cow;
use std::pin::Pin;
use std::sync::Arc;
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
	fn stream_actions(
		&mut self,
		action_store: impl ActionStoreProvider,
		actor: ActorId,
		thread: ThreadId,
	) -> BoxedFuture<'_, Result<ActionStream>>;
}




pub(super) type ResPartialStream =
	Pin<Box<dyn Stream<Item = Result<ResponsePartial>> + Send>>;


/// Processes typed streaming events into [`ActionStreamOut`] values.
pub struct ActionStream {
	action_store: Arc<dyn ActionStoreProvider>,
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
		action_store: Arc<dyn ActionStoreProvider>,
		provider_slug: Cow<'static, str>,
		model_slug: Cow<'static, str>,
		agent: ActorId,
		thread: ThreadId,
		inner: ResPartialStream,
	) -> Self {
		Self {
			action_store,
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
	pub fn actions(&self) -> impl Iterator<Item = &Action> {
		self.actions.values()
	}

	/// Commit the created actions to the ActionStore.
	pub async fn write(mut self) -> Result {
		let actions = self.actions.values().collect::<Vec<_>>();
		let response = self.response.ok_or_else(|| {
			bevyhow!(
				"response id is required to write actions, did the stream finish?"
			)
		})?;

		let metas = actions.iter().map(|action| ResponseMeta {
			action_id: action.id(),
			provider_slug: self.provider_slug.to_string(),
			model_slug: self.model_slug.to_string(),
			response_id: response.response_id.clone(),
			response_stored: response.response_stored,
		});
		self.action_store
			.insert_response_metas(metas.collect())
			.await?;
		self.action_store
			.insert_actions(self.actions.drain().collect())
			.await
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
