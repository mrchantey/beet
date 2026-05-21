use crate::prelude::*;
use beet_core::prelude::*;


/// An [`EntityCommand`] that mutates a token.
///
/// Prefer [`TokenQuery`] when calling from a system, this command is for
/// contexts where direct system access is unavailable (queued from observers,
/// hooks, async tasks, etc).
#[derive(Deref)]
pub struct TokenCommand {
	#[deref]
	pub token: Token,
	pub handler: TokenCommandHandler,
}

impl EntityCommand<Result> for TokenCommand {
	fn apply(self, mut entity: EntityWorldMut) -> Result {
		entity.with_state::<TokenQuery, _>(move |entity, mut query| {
			query.handle_token_command(entity, self)
		})
	}
}

impl TokenCommand {
	/// Mutate the token's value through a closure. Errors if no value is
	/// cached, use [`Self::mutate_or_init`] to seed a value first.
	pub fn mutate_value(
		token: Token,
		handler: impl 'static + Send + Sync + FnOnce(&mut TokenValue) -> Result,
	) -> Self {
		Self {
			token,
			handler: TokenCommandHandler::MutateValue {
				init: None,
				handler: Box::new(handler),
			},
		}
	}

	/// Like [`Self::mutate_value`], but uses `init` to seed the token's value
	/// when it's not yet cached.
	pub fn mutate_or_init(
		token: Token,
		init: impl 'static + Send + Sync + FnOnce() -> Value,
		handler: impl 'static + Send + Sync + FnOnce(&mut TokenValue) -> Result,
	) -> Self {
		Self {
			token,
			handler: TokenCommandHandler::MutateValue {
				init: Some(Box::new(init)),
				handler: Box::new(handler),
			},
		}
	}
}

// In the future we may want to support heavier operations like
// executing arbitrary systems.
pub enum TokenCommandHandler {
	MutateValue {
		/// Seed value used when no entry exists for the token. Leave `None`
		/// to error on missing.
		init: Option<Box<dyn 'static + Send + Sync + FnOnce() -> Value>>,
		handler:
			Box<dyn 'static + Send + Sync + FnOnce(&mut TokenValue) -> Result>,
	},
}
