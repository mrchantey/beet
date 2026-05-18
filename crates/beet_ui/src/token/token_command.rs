use crate::prelude::*;
use beet_core::prelude::*;



#[derive(SystemParam, Get)]
pub struct TokenQuery<'w, 's> {
	commands: Commands<'w, 's>,
	token_set: ResMut<'w, TokenSet>,
}

impl TokenQuery<'_, '_> {
	pub fn handle_token_command(
		&mut self,
		_entity: Entity,
		cmd: TokenCommand,
	) -> Result {
		let TokenCommand { token, handler } = cmd;

		// mutate the cached value in the TokenSet
		let value = {
			let token_value =
				self.token_set.value_mut(&token).ok_or_else(|| {
					bevyhow!(
						"Token not registered in TokenSet\nkey: {}",
						token.key()
					)
				})?;
			match handler {
				TokenCommandHandler::MutateValue(handler) => {
					handler(token_value)?;
				}
			}
			token_value.clone()
		};

		// notify every entity listening on this token with the resolved value
		if let TokenValue::Value(value) = value {
			let value = value.value().clone();
			let listeners: Vec<Entity> = self
				.token_set
				.listeners(&token)
				.map(|listeners| listeners.iter().copied().collect())
				.unwrap_or_default();
			for entity in listeners {
				self.commands.entity(entity).insert(value.clone());
			}
		}
		Ok(())
	}
}

/// An [`EntityCommand`] that mutates a token
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
	pub fn mutate_value(
		token: Token,
		handler: impl 'static + Send + Sync + FnOnce(&mut TokenValue) -> Result,
	) -> Self {
		Self {
			token,
			handler: TokenCommandHandler::MutateValue(Box::new(handler)),
		}
	}
}
// In the future we may want to support heavier operations like
// executing arbitary systems
pub enum TokenCommandHandler {
	MutateValue(
		Box<dyn 'static + Send + Sync + FnOnce(&mut TokenValue) -> Result>,
	),
}
