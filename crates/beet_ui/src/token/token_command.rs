use crate::prelude::*;
use beet_core::prelude::*;



#[derive(SystemParam, Get)]
pub struct TokenQuery<'w, 's> {
	commands: Commands<'w, 's>,
	children: Query<'w, 's, &'static Children>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	classes: Query<'w, 's, &'static Classes>,
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
		// fall back to a shared rule (eg. an inline class rule) that
		// declares this token regardless of selector.
		if let Some(rule) = self.rule_set.find_rule_mut_by_key(token.key()) {
			let value = rule.get_mut(token)?;
			return func(value).xok();
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
		let token_key = I32Value::token_key();
		// find the selector of the rule that maps I32Value -> token
		let selector = self
			.rule_set
			.rules()
			.find(|rule| {
				rule.iter().any(|(key, val)| {
					key == &token_key
						&& matches!(val, TokenValue::Token(t) if t == token)
				})
			})
			.map(|rule| rule.selector().clone());
		let Some(selector) = selector else {
			return;
		};
		let children: Vec<Entity> =
			self.children.iter_descendants_inclusive(entity).collect();
		for child in children {
			let matches = match &selector {
				Selector::Entity(e) => *e == child,
				Selector::Class(class) => self
					.classes
					.get(child)
					.map(|c| c.contains_selector(class))
					.unwrap_or(false),
				_ => false,
			};
			if matches {
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
