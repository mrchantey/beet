use crate::prelude::*;
use beet_core::prelude::*;

// #[derive(Default)]
// pub struct TokenPlugin;

// impl Plugin for TokenPlugin {
// 	fn build(&self, app: &mut App) {
// 		// app.add_observer(handle_token_event);
// 		// app.add_systems(PostUpdate, apply_values);
// 	}
// }

// fn handle_token_event(ev: On<TokenEvent>, mut query: TokenQuery) -> Result {
// 	query.handle_token_event(&ev)
// }

#[derive(SystemParam, Get)]
pub struct TokenQuery<'w, 's> {
	commands: Commands<'w, 's>,
	// entity_tokens: Query<'w, 's, (Entity, &'static TokenStore)>,
	global_tokens: Option<Res<'w, TokenStore>>,
	children: Query<'w, 's, &'static Children>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	stores: Query<'w, 's, &'static mut TokenStore>,
}

impl TokenQuery<'_, '_> {
	pub fn handle_token_event(
		&mut self,
		ev_entity: Entity,
		ev: TokenCommand,
	) -> Result {
		for entity in self.ancestors.iter_ancestors_inclusive(ev_entity) {
			let Ok(mut store) = self.stores.get_mut(entity) else {
				continue;
			};
			// avoid triggering change detection
			if !store.contains_key(ev.key()) {
				continue;
			}
			let value = store.get_mut(&ev.token).unwrap();
			match ev.handler {
				TokenEventHandler::MutateValue(handler) => {
					handler(value)?;
				}
			}
			// ideally we wouldnt need to clone, but need mutable access
			// to other stores to apply
			if let TokenValue::Value(value) = value {
				let value = value.value().clone();
				self.apply_value(entity, &ev.token, value);
			}
			return Ok(());
		}
		bevybail!(
			"Token not found in entity or ancestors\nkey: {}\nentity: {:?}",
			ev.key(),
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
	pub handler: TokenEventHandler,
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
			handler: TokenEventHandler::MutateValue(Box::new(handler)),
		}
	}
}

pub enum TokenEventHandler {
	MutateValue(
		Box<dyn 'static + Send + Sync + FnOnce(&mut TokenValue) -> Result>,
	),
}

fn handle_token_event(
	In((entity, ev)): In<(Entity, TokenCommand)>,
	mut query: TokenQuery,
) -> Result {
	query.handle_token_event(entity, ev)
}
