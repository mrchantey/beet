use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use std::pin::Pin;




pub trait ActionStreamer {
	/// Streamers like the [`OrStreamer`] support caching
	/// previously sent items, this allows them to opt into
	/// specifying which point in the thread they are up to,
	/// for preloading into the [`ContextMap`] before calling
	/// [`stream_actions`].
	fn last_action_sent(&self) -> Option<ActionId>;
	fn stream_actions(
		&mut self,
		actor: ActorId,
		thread: ThreadId,
		context_map: &ContextMap,
	) -> BoxedFuture<'_, Result<ActionStream>>;
}

pub type ActionStream = Pin<
	Box<dyn Stream<Item = Result<Vec<PartialItem>, ActionStreamError>> + Send>,
>;


#[derive(Debug, thiserror::Error)]
pub enum ActionStreamError {
	#[error("stream interrupted")]
	Interrupted,
	#[error("{0}")]
	Opaque(BevyError),
}
