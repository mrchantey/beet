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
	action_map: DocMap<Action>,
	action_partial_map: ActionPartialMap,
}

impl ActionStream {
	pub fn new(
		action_store: Arc<dyn ActionStoreProvider>,
		agent: ActorId,
		thread: ThreadId,
		inner: ResPartialStream,
	) -> Self {
		Self {
			action_store,
			agent,
			thread,
			inner,
			action_map: default(),
			action_partial_map: default(),
		}
	}

	/// Returns an iterator over the collected actions.
	pub fn actions(&self) -> impl Iterator<Item = &Action> {
		self.action_map.values()
	}

	/// Commit the created actions to the ActionStore.
	pub async fn write(self) -> Result {
		self.action_store
			.insert_actions(&self.action_map.values().collect::<Vec<_>>())
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
			Poll::Ready(Some(Ok(res_partial))) => {
				trace!("Streaming Event: {:#?}", res_partial);
				// get next state without the actions
				let changes = this.action_partial_map.apply_actions(
					&mut this.action_map,
					this.agent,
					this.thread,
					res_partial.actions,
				)?;

				Poll::Ready(Some(Ok(changes)))
			}
			Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
