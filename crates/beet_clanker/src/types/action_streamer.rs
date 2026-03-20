use crate::prelude::*;
use beet_core::prelude::*;
use bevy::tasks::BoxedFuture;
use futures::Stream;
use std::borrow::Cow;
use std::pin::Pin;



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
		actor: ActorId,
		thread: ThreadId,
	) -> BoxedFuture<'_, Result<ActionStream>>;
}

pub type ActionStream =
	Pin<Box<dyn Stream<Item = Result<ActionStreamOut>> + Send>>;




#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionStreamOut {
	state: ActionStreamState,
	mutations: Vec<(ActionId, ActionMutationKind)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionStreamState {
	Created,
	Queued,
	InProgress,
	Completed,
	Failed,
	Incomplete,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionMutationKind {
	Created,
	Updated,
}
