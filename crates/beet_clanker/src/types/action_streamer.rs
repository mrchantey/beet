use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;
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
	fn stream_actions(
		&mut self,
		store: impl ActionStoreProvider,
		actor: ActorId,
		thread: ThreadId,
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

	/// Commit the specified actions to the store,
	/// ignoring any not in its map.
	pub async fn write(
		&self,
		store: impl ActionStoreProvider,
		actions: Vec<ActionId>,
	) -> Result {
		let actions = self
			.actions
			.values()
			.filter(|action| actions.contains(&&action.id()))
			.map(|action| action.clone())
			.collect::<Vec<_>>();
		let response = self.response.as_ref().ok_or_else(|| {
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
		store.insert_response_metas(metas.collect()).await?;
		store.insert_actions(actions).await?;
		Ok(())
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
