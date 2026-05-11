use crate::prelude::*;
use beet_core::prelude::*;



#[derive(SystemParam, Get)]
pub struct TokenQuery<'w, 's> {
	commands: Commands<'w, 's>,
	children: Query<'w, 's, &'static Children>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	stores: Query<'w, 's, &'static mut Rule>,
}

impl TokenQuery<'_, '_> {
	pub fn handle_token_command(
		&mut self,
		entity: Entity,
		cmd: TokenCommand,
	) -> Result {
		// ideally we wouldnt need to clone, but need mutable access
		// to other stores to apply
		let value = self.with_value_mut(
			entity,
			&cmd.token,
			move |value| -> Result<TokenValue> {
				match cmd.handler {
					TokenCommandHandler::MutateValue(handler) => {
						handler(value)?;
					}
				}
				value.clone().xok()
			},
		)??;
		if let TokenValue::Value(value) = value {
			let value = value.value().clone();
			self.apply_value(entity, &cmd.token, value);
		}
		Ok(())
	}

	/// Walk up the entity tree to find the first [`Rule`] component with the
	/// provided token. Analogous to CSS variable inheritance.
	fn with_value_mut<O>(
		&mut self,
		ev_entity: Entity,
		token: &Token,
		func: impl FnOnce(&mut TokenValue) -> O,
	) -> Result<O> {
		for entity in self.ancestors.iter_ancestors_inclusive(ev_entity) {
			let Ok(mut store) = self.stores.get_mut(entity) else {
				continue;
			};
			// avoid triggering change detection
			if !store.contains_key(token.key()) {
				continue;
			}
			let value = store.get_mut(&token).unwrap();
			return func(value).xok();
		}
		bevybail!(
			"Token not found in entity or ancestors\nkey: {}\nentity: {:?}",
			token.key(),
			ev_entity
		)
	}

	// this is so stupid
	pub fn apply_value(&mut self, entity: Entity, token: &Token, value: Value) {
		for child in self.children.iter_descendants_inclusive(entity) {
			if let Ok(mut store) = self.stores.get_mut(child) {
				for (key, store_value) in store.iter_mut() {
					if let TokenValue::Token(store_value) = store_value {
						if store_value == token {
							if key == &I32Value::token_key() {
								self.commands
									.entity(child)
									.insert(value.clone());
							}
						}
					}
				}
			}
		}
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
	fn apply(self, entity: EntityWorldMut) -> Result {
		let id = entity.id();
		entity
			.into_world_mut()
			.run_system_cached_with::<_, Result, _, _>(
				handle_token_event,
				(id, self),
			)??;
		Ok(())
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
// In the future we may want to support 'heavier' operations like
// arbitary systems
pub enum TokenCommandHandler {
	MutateValue(
		Box<dyn 'static + Send + Sync + FnOnce(&mut TokenValue) -> Result>,
	),
}

fn handle_token_event(
	In((entity, ev)): In<(Entity, TokenCommand)>,
	mut query: TokenQuery,
) -> Result {
	query.handle_token_command(entity, ev)
}
