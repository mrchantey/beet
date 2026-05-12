use crate::prelude::*;
use beet_core::prelude::*;



#[derive(SystemParam, Get)]
pub struct TokenQuery<'w, 's> {
	commands: Commands<'w, 's>,
	children: Query<'w, 's, &'static Children>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	rule_set: ResMut<'w, RuleSet>,
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

	/// Walk up the entity tree to find the first [`RuleSet`] rule with
	/// `Selector::Entity(entity)` that contains the provided token.
	/// Analogous to CSS variable inheritance.
	fn with_value_mut<O>(
		&mut self,
		ev_entity: Entity,
		token: &Token,
		func: impl FnOnce(&mut TokenValue) -> O,
	) -> Result<O> {
		// collect first to avoid holding a borrow on self.ancestors
		// while also borrowing self.rule_set
		let ancestors: Vec<Entity> =
			self.ancestors.iter_ancestors_inclusive(ev_entity).collect();
		for entity in ancestors {
			if let Some(rule) =
				self.rule_set.find_entity_rule_mut(entity, token.key())
			{
				let value = rule.get_mut(token).unwrap();
				return func(value).xok();
			}
		}
		bevybail!(
			"Token not found in entity or ancestors\nkey: {}\nentity: {:?}",
			token.key(),
			ev_entity
		)
	}

	pub fn apply_value(&mut self, entity: Entity, token: &Token, value: Value) {
		// collect first to avoid holding borrows on self.children and
		// self.rule_set simultaneously with self.commands
		let children: Vec<Entity> =
			self.children.iter_descendants_inclusive(entity).collect();
		let token_key = I32Value::token_key();
		for child in children {
			let selector = Selector::Entity(child);
			let should_insert = self.rule_set.rules().any(|rule| {
				rule.selector() == &selector
					&& rule.iter().any(|(key, val)| {
						matches!(val, TokenValue::Token(t) if t == token)
							&& key == &token_key
					})
			});
			if should_insert {
				self.commands.entity(child).insert(value.clone());
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
