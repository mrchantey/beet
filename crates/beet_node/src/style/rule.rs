use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// A set of default properties applied to elements matching the given criteria.
#[derive(Debug, Default, Clone, Reflect, Get)]
pub struct Rule {
	/// All the rules an element must match for styles to be applied.
	/// Empty matches all elements
	rules: Vec<Selector>,
	declarations: HashMap<TokenKey, TokenValue>,
}


impl Rule {
	pub fn root() -> Self { Self::new().with_rule(Selector::Root) }

	/// Match elements with the given tag.
	pub fn new() -> Self { Self::default() }

	pub fn with_token<K: TypedTokenKey, V: TypedToken>(self) -> Self {
		self.with(K::token_key(), V::token())
	}
	pub fn with_value<K: TypedTokenKey>(
		self,
		value: impl Typed,
	) -> Result<Self> {
		self.with(K::token_key(), TypedValue::new(value)?).xok()
	}
	/// Add a property mapped to a token.
	pub fn with(
		mut self,
		token: TokenKey,
		value: impl Into<TokenValue>,
	) -> Self {
		self.declarations.insert(token, value.into());
		self
	}

	pub fn with_rule(mut self, rule: Selector) -> Self {
		self.rules.push(rule);
		self
	}
	/// Matches all rules, or `true` if empty
	pub fn matches(&self, el: &ElementView) -> bool {
		self.rules.iter().all(|rule| rule.matches(el))
	}
}

// akin to a lightningcss Component, ie `/selectors/parser.rs#1392`
/// A match rule
#[derive(Debug, Clone, Reflect)]
pub enum Selector {
	/// A global selector, in css this will evaluate to `:root`,
	/// and will always match true
	Root,
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
	Not(Vec<Selector>),
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
	pub fn not(rules: Vec<Selector>) -> Self { Self::Not(rules) }

	pub fn matches(&self, el: &ElementView) -> bool {
		match self {
			Selector::Root => true,
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
			Selector::Not(inner) => {
				!inner.iter().any(|rule| rule.matches(el))
			}
		}
	}
}
