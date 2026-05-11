use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::collections::VecDeque;


#[derive(Resource)]
pub struct RuleSet {
	rules: VecDeque<Rule>,
}

/// By default, the rule set is initialized with a single root rule
impl Default for RuleSet {
	fn default() -> Self { Self::new(default()) }
}


impl RuleSet {
	pub fn new(default_rule: Rule) -> Self {
		let mut rules = VecDeque::with_capacity(1);
		rules.push_back(default_rule);
		Self { rules }
	}
	/// Add a new rule, merging with the last added if selectors match
	pub fn insert_rule(&mut self, rule: Rule) {
		if let Some(last) = self.rules.back_mut() {
			if last.selector() == rule.selector() {
				last.push_declarations(rule);
				return;
			}
		} else {
			self.rules.push_back(rule);
		}
	}
	pub fn with_rule(mut self, rule: Rule) -> Self {
		self.insert_rule(rule);
		self
	}

	/// Gets the first added rule, by default this
	/// is a rule with a root selector
	pub fn default_rule(&self) -> &Rule {
		self.rules
			.front()
			.expect("RuleSet should have at least one rule")
	}
	/// Gets the first added rule, by default this
	/// is a rule with a root selector
	pub fn default_rule_mut(&mut self) -> &mut Rule {
		self.rules
			.front_mut()
			.expect("RuleSet should have at least one rule")
	}
	pub fn insert(
		&mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<&mut Self> {
		self.default_rule_mut().insert(key, value)?;
		self.xok()
	}
	fn with(
		mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<Self> {
		self.insert(key, value)?;
		self.xok()
	}

	pub fn with_token(
		self,
		key: impl Into<Token>,
		value: impl Into<Token>,
	) -> Result<Self> {
		self.with(key, value)
	}
	pub fn with_value(
		self,
		key: impl Into<Token>,
		value: impl Typed + Serialize,
	) -> Result<Self> {
		self.with(key, TypedValue::new(value)?)
	}
	#[track_caller]
	pub fn with_inline_value<T>(self, value: T) -> Result<Self>
	where
		T: Typed + Serialize,
	{
		let key = Token::new_inline(TokenSchema::of::<T>());
		self.with(key, TypedValue::new(value)?)
	}

	fn cascade(&self, el: &ElementView, key: &Token) -> Result<&TokenValue> {
		self.rules
			.iter()
			.filter(|rule| rule.selector().matches(el))
			.xtry_find_map(|rule| rule.declarations().get(key))
	}
}


#[derive(SystemParam)]
pub struct RuleSetQuery<'w, 's> {
	rule_set: ResMut<'w, RuleSet>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	_children: Query<'w, 's, &'static Children>,
	element_query: ElementQuery<'w, 's>,
}

impl RuleSetQuery<'_, '_> {
	pub fn resolve(&self, entity: Entity, token: &Token) -> Result<&Value> {
		match self.cascade(entity, token) {
			Ok(TokenValue::Value(value)) => value.value().xok(),
			Ok(TokenValue::Token(token)) => self.resolve(entity, &token),
			Err(err) if !token.inherited() => {
				// dont look in ancestors for non-inherited tokens
				Err(err)
			}
			Err(err) => {
				if let Ok(ancestor) =
					self.ancestors.get(entity).map(|ancestor| ancestor.get())
				{
					self.resolve(ancestor, token)
				} else {
					Err(err)
				}
			}
		}
	}
	pub fn cascade(
		&self,
		entity: Entity,
		token: &Token,
	) -> Result<&TokenValue> {
		let el = self.element_query.get(entity)?;
		let value = self.rule_set.cascade(&el, token)?;
		Ok(value)
	}
}





#[cfg(test)]
mod tests {
	use super::*;

	token!(Foo, u32);
	token!(Bar, u32);

	#[test]
	fn cascade() {
		let mut world = World::new();
		world.insert_resource(
			RuleSet::default()
				.with_token(Foo, Bar)
				.unwrap()
				.with_value(Bar, 3u32)
				.unwrap(),
		);
		let mut entity = world.spawn(rsx! {<div/>});
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.cascade(entity, &Foo.into()).cloned()
			})
			.unwrap()
			.xpect_eq(TokenValue::token(Bar));
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.cascade(entity, &Bar.into()).cloned()
			})
			.unwrap()
			.xpect_eq(TokenValue::value(3u32).unwrap());
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.resolve(entity, &Bar.into()).cloned()
			})
			.unwrap()
			.xpect_eq(3u32.into());
	}
}
