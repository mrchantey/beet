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
	Pin<Box<dyn Stream<Item = Result<ActionStreamState>> + Send>>;




#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionStreamState {
	pub response_id: String,
	/// whether the response was stored, enabling usage of
	/// `previous_response_id` in the next request.
	pub response_stored: bool,
	pub status: ActionStreamStatus,
	pub token_usage: Option<TokenUsage>,
	/// List of actions that were inserted, these may have
	/// been created or updated.
	pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionStreamStatus {
	Created,
	Queued,
	InProgress,
	Completed,
	Failed {
		code: Option<String>,
		message: Option<String>,
	},
	Incomplete(Option<String>),
	Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenUsage {
	/// The number of input tokens used to generate the response.
	pub input_tokens: u32,
	/// The number of input tokens used to generate the response.
	pub output_tokens: u32,
	/// The total number of tokens used.
	pub total_tokens: u32,
	/// The number of input tokens that were served from cache.
	pub cached_input_tokens: Option<u32>,
	/// The number of output tokens attributed to reasoning.
	pub reasoning_tokens: Option<u32>,
}
