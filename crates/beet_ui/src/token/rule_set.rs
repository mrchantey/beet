use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::collections::VecDeque;

/// Global store of style [`Rule`]s.
///
/// Holds an ordered list of matching rules plus a single `:root` default rule.
/// The default rule is the **lowest-priority fallback**: the cascade only
/// consults it (via [`RuleSetQuery`]) once the matching rules and the ancestor
/// walk find nothing, so a matching rule like `.dark-scheme` always overrides a
/// value baked into `:root`. Among the matching rules, earlier entries win ties
/// (they're ordered most-specific first).
#[derive(Debug, Clone, Reflect, Resource)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RuleSet {
	/// The `:root` rule — the lowest-priority fallback, kept out of `rules` so
	/// it never shadows a matching rule.
	default_rule: Rule,
	/// Ordered matching rules; earlier rules win ties.
	rules: VecDeque<Rule>,
	/// Inline rules are only declared once. Calling [`Self::try_insert_inline`]
	/// with a rule whose selector matches one of these does nothing.
	registered_inline: HashSet<Selector>,
}

/// By default, the rule set is initialized with an empty `:root` rule.
impl Default for RuleSet {
	fn default() -> Self { Self::new(default()) }
}


impl RuleSet {
	pub fn new(default_rule: Rule) -> Self {
		Self {
			default_rule,
			rules: VecDeque::new(),
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

	/// Add a new rule, merging with the last added when both its selector and
	/// `@media` gate match. The media check keeps a screen/terminal-gated rule
	/// from folding its declarations into an adjacent ungated rule with the same
	/// selector (eg `.sidebar` + a screen-only `.sidebar` width), which would
	/// strip the gate and leak the value to every target.
	pub fn insert_rule(&mut self, rule: Rule) {
		if let Some(last) = self.rules.back_mut()
			&& last.selector() == rule.selector()
			&& last.media() == rule.media()
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

	/// Iterates the matching rules in insertion order, excluding the `:root`
	/// default rule.
	pub fn rules(&self) -> impl Iterator<Item = &Rule> { self.rules.iter() }

	/// Iterates every rule for serialization — the `:root` default first, then
	/// the matching rules.
	pub fn iter(&self) -> impl Iterator<Item = &Rule> {
		core::iter::once(&self.default_rule).chain(self.rules.iter())
	}

	/// The `:root` default rule — the lowest-priority cascade fallback.
	pub fn default_rule(&self) -> &Rule { &self.default_rule }
	/// Mutable access to the `:root` default rule.
	pub fn default_rule_mut(&mut self) -> &mut Rule { &mut self.default_rule }
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
	#[cfg(feature = "serde")]
	pub fn with_value(
		self,
		key: impl Into<Token>,
		value: impl Typed + Serialize,
	) -> Result<Self> {
		self.with(key, TypedValue::new(value)?)
	}
	#[cfg(feature = "serde")]
	#[track_caller]
	pub fn with_inline_value<T>(self, value: T) -> Result<Self>
	where
		T: Typed + Serialize,
	{
		let key = Token::new_inline(FieldSchema::of::<T>());
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
		// The `:root` default rule is the lowest-priority fallback,
		// applied by `RuleSetQuery` after the ancestor walk, so a matching rule
		// (eg `.dark-scheme`) can override a `:root` default, mirroring CSS.
		// `@media`-gated rules are skipped *unless* gated by `Terminal`, the one
		// query whose context is this cascade: print/screen/reduced-motion are
		// web concerns that only affect CSS output, while a `Terminal` rule (eg
		// the colored prose headings) applies here and is excluded from CSS.
		// The most specific matching rule wins (class beats tag); ties go to the
		// later rule, mirroring CSS source order (and the serialized stylesheet)
		// so a theme override appended after a user-agent default wins on both.
		self.rules
			.iter()
			.filter(|rule| {
				rule.media().is_none_or(MediaQuery::is_terminal)
					&& rule.selector().matches(el)
			})
			.filter_map(|rule| {
				rule.get(key)
					.ok()
					.map(|value| (rule.selector().specificity(), value))
			})
			.reduce(|best, next| if next.0 >= best.0 { next } else { best })
			.map(|(_, value)| value)
			.ok_or_else(|| bevyhow!("no matching rule for token `{key}`"))
	}
}


#[derive(SystemParam)]
pub struct RuleSetQuery<'w, 's> {
	rule_set: ResMut<'w, RuleSet>,
	ancestors: Query<'w, 's, &'static ChildOf>,
	_children: Query<'w, 's, &'static Children>,
	// reverse [`RenderRef`] lookup, so the inherited cascade crosses transclusion
	// boundaries: content transcluded into a layout by reference has no `ChildOf`
	// edge to the layout, so inheritance (eg the color scheme) continues from the
	// holder that renders it in place.
	render_refs: Query<'w, 's, (Entity, &'static RenderRef)>,
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
			Err(err) => {
				// inherited tokens search ancestors before the root fallback
				if token.is_inherited()
					&& let Some(ancestor) = self.parent(entity)
				{
					self.resolve_untyped(ancestor, token)
				} else {
					// fall back to the `:root` default declarations
					self.resolve_default(entity, token).map_err(|_| err)
				}
			}
		}
	}

	/// The cascade parent of `entity`. A transcluded entity (the target of a
	/// [`RenderRef`]) inherits from the holder that renders it in place, not from
	/// its original [`ChildOf`] spawn location — so the cascade (eg the color
	/// scheme) crosses the transclusion boundary. Otherwise the `ChildOf` parent.
	fn parent(&self, entity: Entity) -> Option<Entity> {
		self.render_refs
			.iter()
			.find(|(_, render_ref)| render_ref.0 == entity)
			.map(|(holder, _)| holder)
			.or_else(|| self.ancestors.get(entity).map(|child_of| child_of.get()).ok())
	}

	/// Resolves `token` against the `:root` default rule — the lowest-priority
	/// fallback consulted once the cascade and ancestor walk find nothing.
	fn resolve_default(&self, entity: Entity, token: &Token) -> Result<&Value> {
		match self.rule_set.default_rule().get(token)? {
			TokenValue::Value(value) => value.value().xok(),
			TokenValue::Token(token) => self.resolve_untyped(entity, token),
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
		// `Bar`'s value lives in the `:root` default rule (the lowest-priority
		// fallback); `Foo` points at `Bar` from a matching rule.
		world.insert_resource(
			RuleSet::default().with_value(Bar, 3u32).unwrap().with_rule(
				Rule::new()
					.with_selector(Selector::Any)
					.with_token(Foo, Bar)
					.unwrap(),
			),
		);
		let mut entity = world.spawn(rsx! { <div/> });

		// a matching (non-default) rule is found directly by `cascade`
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.cascade(entity, &Foo.into()).cloned()
			})
			.unwrap()
			.xpect_eq(TokenValue::token(Bar));

		// `Bar` lives only in the `:root` default rule, which `cascade` excludes ...
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.cascade(entity, &Bar.into()).is_err()
			})
			.xpect_true();

		// ... but resolution falls back to it, following the token chain
		entity
			.with_state::<RuleSetQuery, _>(|entity, query| {
				query.resolve_untyped(entity, &Foo.into()).cloned()
			})
			.unwrap()
			.xpect_eq(3u32.into());
	}
}
