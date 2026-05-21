use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::collections::VecDeque;

/// Global store of an ordered [`Rule`] list. Rules contain
/// tokens that may or may not apply to an [`Element`].
#[derive(Debug, Clone, Reflect, Deref, DerefMut, Resource)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RuleSet {
	#[deref]
	rules: VecDeque<Rule>,
	/// Inline rules are only declared once. Calling [`Self::try_insert_inline`]
	/// with a rule whose selector matches one of these does nothing.
	registered_inline: HashSet<Selector>,
}

/// By default, the rule set is initialized with a single root rule
impl Default for RuleSet {
	fn default() -> Self { Self::new(default()) }
}


impl RuleSet {
	pub fn new(default_rule: Rule) -> Self {
		let mut rules = VecDeque::with_capacity(1);
		rules.push_back(default_rule);
		Self {
			rules,
			registered_inline: default(),
		}
	}

	/// Attempt to register an inline rule. If a rule with the same selector
	/// has already been registered this does nothing and returns `false`,
	/// otherwise the rule is inserted and `true` is returned.
	pub fn try_insert_inline(&mut self, rule: Rule) -> bool {
		if self.registered_inline.contains(rule.selector()) {
			return false;
		}
		self.registered_inline.insert(rule.selector().clone());
		self.insert_rule(rule);
		true
	}

	/// Find the first rule containing `key`, regardless of selector.
	pub fn find_rule_mut_by_key(
		&mut self,
		key: &TokenKey,
	) -> Option<&mut Rule> {
		self.rules.iter_mut().find(|r| r.contains_key(key))
	}
	/// Add a new rule, merging with the last added if selectors match
	pub fn insert_rule(&mut self, rule: Rule) {
		if let Some(last) = self.rules.back_mut()
			&& last.selector() == rule.selector()
		{
			last.push_declarations(rule);
		} else {
			self.rules.push_back(rule);
		}
	}
	pub fn with_rule(mut self, rule: Rule) -> Self {
		self.insert_rule(rule);
		self
	}
	/// Inserts multiple rules.
	pub fn with_rules(mut self, rules: impl IntoIterator<Item = Rule>) -> Self {
		for rule in rules {
			self.insert_rule(rule);
		}
		self
	}

	/// Find the first rule matching `Selector::Entity(entity)` that contains `key`.
	pub fn find_entity_rule_mut(
		&mut self,
		entity: Entity,
		key: &TokenKey,
	) -> Option<&mut Rule> {
		self.rules.iter_mut().find(|r| {
			r.selector() == &Selector::Entity(entity) && r.contains_key(key)
		})
	}

	/// Iterates all rules in insertion order.
	pub fn rules(&self) -> impl Iterator<Item = &Rule> { self.rules.iter() }

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

	/// Extend with multiple rules, inserting each (merging when selectors match).
	pub fn extend_rules(
		&mut self,
		rules: impl IntoIterator<Item = Rule>,
	) -> &mut Self {
		for rule in rules {
			self.insert_rule(rule);
		}
		self
	}

	fn cascade(&self, el: &ElementView, key: &Token) -> Result<&TokenValue> {
		self.rules
			.iter()
			.filter(|rule| rule.selector().matches(el))
			.xtry_find_map(|rule| rule.get(key))
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
	pub fn resolve<T>(&self, entity: Entity, token: T) -> Result<T::Value>
	where
		T: TypedToken + Into<Token>,
		T::Value: DeserializeOwned,
	{
		self.resolve_untyped(entity, &token.into())
			.and_then(|value| value.clone().into_serde::<T::Value>())
	}
	pub fn resolve_untyped(
		&self,
		entity: Entity,
		token: &Token,
	) -> Result<&Value> {
		match self.cascade(entity, &token) {
			Ok(TokenValue::Value(value)) =>
			// mapped directly to value, ie background-color: green
			{
				value.value().xok()
			}
			Ok(TokenValue::Token(token)) => {
				// points to another token ie background-color: primary
				self.resolve_untyped(entity, &token)
			}
			Err(err) if !token.is_inherited() => {
				// dont look in ancestors for non-inherited tokens
				Err(err)
			}
			Err(err) => {
				if let Ok(ancestor) =
					self.ancestors.get(entity).map(|ancestor| ancestor.get())
				{
					self.resolve_untyped(ancestor, token)
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
		// get the nearest ancestor element, handling
		// text and fragment nodes
		let el = self.element_query.get_in_ancestors(entity)?;
		let value = self.rule_set.cascade(&el, token)?;
		Ok(value)
	}
}





#[cfg(test)]
mod tests {
	use super::*;

	token!(Foo, u32);
	token!(Bar, u32);

	#[beet_core::test]
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
				query.resolve_untyped(entity, &Bar.into()).cloned()
			})
			.unwrap()
			.xpect_eq(3u32.into());
	}
}
