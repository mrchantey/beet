use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::sync::Arc;

/// A set of declarations applied to elements matching the given selector.
#[derive(Debug, Default, Clone, Reflect, Component, Get, GetMut, SetWith)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rule {
	/// Predicate for which entities this rule applies to
	selector: Selector,
	declarations: HashMap<TokenKey, TokenValue>,
}

impl Rule {
	pub fn new() -> Self { Self::default() }

	/// Create a rule with a class selector, eg `Rule::class("btn-filled")`.
	pub fn class(class: &str) -> Self {
		Self {
			selector: Selector::Class(class.into()),
			declarations: default(),
		}
	}

	/// Create a rule with a tag selector, eg `Rule::tag("button")`.
	pub fn tag(tag: &str) -> Self {
		Self {
			selector: Selector::Tag(tag.into()),
			declarations: default(),
		}
	}

	pub fn insert(
		&mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<&mut Self> {
		let value = value.into();
		let key = key.into();
		key.schema().assert_eq(value.schema())?;
		self.declarations.insert(key.key().clone(), value);
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

	pub fn with_value<T>(self, key: T, value: impl Into<T::Value>) -> Self
	where
		T: TypedToken + Into<Token>,
		T::Value: Typed + Serialize,
	{
		self.with(
			key,
			TypedValue::new(value.into())
				.expect("failed to serialize typed value"),
		)
		.expect(
			"Schema assertion failed for a typed value, this shouldnt be possible",
		)
	}
	pub fn with_value_untyped(
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

	/// Extend declarations from an iterator of `(TokenKey, TokenValue)` pairs.
	pub fn with_extend(
		mut self,
		iter: impl IntoIterator<Item = (TokenKey, TokenValue)>,
	) -> Self {
		self.declarations.extend(iter);
		self
	}

	/// Get a declaration value by token, performing schema validation.
	pub fn get(&self, token: &Token) -> Result<&TokenValue> {
		match self.declarations.get(token.key()) {
			Some(value) => {
				token.schema().assert_eq(value.schema())?;
				Ok(value)
			}
			None => bevybail!("Token Not Found: `{token}`"),
		}
	}

	pub fn get_mut(&mut self, token: &Token) -> Result<&mut TokenValue> {
		match self.declarations.get_mut(token.key()) {
			Some(value) => {
				token.schema().assert_eq(value.schema())?;
				Ok(value)
			}
			None => bevybail!("Token Not Found: `{token}`"),
		}
	}

	pub fn contains_key(&self, key: &TokenKey) -> bool {
		self.declarations.contains_key(key)
	}

	/// Iterate over all declarations.
	pub fn iter(&self) -> impl Iterator<Item = (&TokenKey, &TokenValue)> {
		self.declarations.iter()
	}

	/// Iterate mutably over all declarations.
	pub fn iter_mut(
		&mut self,
	) -> impl Iterator<Item = (&TokenKey, &mut TokenValue)> {
		self.declarations.iter_mut()
	}

	/// Merge another rule's declarations into this one (builder pattern).
	pub fn extend_declarations(mut self, other: Self) -> Self {
		self.declarations.extend(other.declarations);
		self
	}

	/// Mutable: merge another rule's declarations into self.
	pub fn push_declarations(&mut self, other: Self) -> &mut Self {
		self.declarations.extend(other.declarations);
		self
	}

	/// Insert a definition if it doesn't already exist.
	pub fn insert_definition<T>(
		&mut self,
		definition: TokenDefinition<T>,
	) -> Result<&mut Self> {
		if self.contains_key(definition.token.key()) {
			bevybail!(
				"Token `{}` already exists in rule declarations",
				definition.token
			);
		}
		self.insert(definition.token.clone(), definition.initial.clone())
	}

	/// Get a typed value, performing schema and type validation.
	#[cfg(feature = "serde")]
	pub fn get_typed<T: Typed + serde::de::DeserializeOwned>(
		&self,
		key: &Token,
	) -> Result<T> {
		key.schema().assert_eq_ty::<T>()?;
		match self.get(key)? {
			TokenValue::Value(value) => value.into_typed::<T>(),
			TokenValue::Token(_) => {
				bevybail!("Expected Value, found Token: `{key}`")
			}
		}
	}

	pub fn merge_any(mut self, other: Self) -> Self {
		self.selector = self.selector.clone().merge_any(other.selector);
		self.declarations.extend(other.declarations);
		self
	}

	/// Matches all rules, or `true` if empty
	pub fn matches(&self, el: &ElementView) -> bool {
		self.selector.matches(el)
	}
}

impl IntoIterator for Rule {
	type Item = (TokenKey, TokenValue);
	type IntoIter =
		bevy::platform::collections::hash_map::IntoIter<TokenKey, TokenValue>;
	fn into_iter(self) -> Self::IntoIter { self.declarations.into_iter() }
}

// akin to a lightningcss Component, ie `/selectors/parser.rs#1392`
/// A match rule
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Selector {
	/// A global predicate, in css this will evaluate to `:root`,
	/// and in bevy apps will always pass predicates
	#[default]
	Root,
	/// Only match a specific entity
	Entity(Entity),
	/// Selects any element, in css this will evaluate to `*`,
	/// and in bevy apps will always pass predicates
	Any,
	/// Match any of the rules, eg `div, .my-class` (note the comma) in css
	AnyOf(Vec<Selector>),
	/// Match all of the rules, eg `div.my-class` (note no comma) in css
	AllOf(Vec<Selector>),
	/// Must have this tag, eg `div`
	Tag(SmolStr),
	/// Must have this class, eg `.my-class`
	Class(SmolStr),
	/// Must be in this state, eg `:hover`
	State(ElementState),
	/// Must have the attribute, ie 'display=flex'
	Attribute {
		key: SmolStr,
		/// Optionally also
		value: Option<Value>,
	},
	/// Negate a rule, ie must not have tag
	Not(Arc<Self>),
}

impl Selector {
	pub fn class(class: impl Into<SmolStr>) -> Self {
		Self::Class(class.into())
	}
	pub fn tag(tag: impl Into<SmolStr>) -> Self { Self::Tag(tag.into()) }
	pub fn state(state: ElementState) -> Self { Self::State(state) }

	pub fn attribute(key: impl Into<SmolStr>, value: Option<Value>) -> Self {
		Self::Attribute {
			key: key.into(),
			value,
		}
	}
	pub fn not(inner: Selector) -> Self { Self::Not(Arc::new(inner)) }

	/// Merge two selectors as an AnyOf, collapsing global selectors
	pub fn merge_any(self, other: Self) -> Self {
		if self == other {
			return self;
		}
		match (self, other) {
			(Self::Any, _) | (_, Self::Any) => Self::Any,
			(Self::AnyOf(mut rules), Self::AnyOf(other)) => {
				rules.extend(other);
				Self::AnyOf(rules)
			}
			(Self::AnyOf(mut rules), r) | (r, Self::AnyOf(mut rules)) => {
				rules.push(r);
				Self::AnyOf(rules)
			}
			(r1, r2) => Self::AnyOf(vec![r1, r2]),
		}
	}

	pub fn matches(&self, el: &ElementView) -> bool {
		match self {
			Selector::Root => true,
			Selector::Any => true,
			Selector::Entity(entity) => el.entity == *entity,
			Selector::AnyOf(rules) => rules.iter().any(|rule| rule.matches(el)),
			Selector::AllOf(rules) => rules.iter().all(|rule| rule.matches(el)),
			Selector::Tag(tag) => el.element.tag() == tag,
			Selector::Attribute { key, value } => match value {
				Some(expected) => el
					.attribute(key)
					.map(|attr| attr.value == expected)
					.unwrap_or(false),
				None => el.attribute(key).is_some(),
			},
			Selector::State(state) => el.contains_state(state),
			Selector::Class(class) => el.contains_class(class),
			Selector::Not(inner) => !inner.matches(el),
		}
	}
}
