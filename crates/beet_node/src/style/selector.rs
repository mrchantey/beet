use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// A set of default properties applied to elements matching the given criteria.
#[derive(Default, Reflect, Get)]
pub struct Selector {
	/// All the rules an element must match for styles to be applied.
	/// Empty matches all elements
	rules: Vec<Rule>,
	tokens: HashMap<TokenKey, TokenValue>,
}


impl Selector {
	pub fn root() -> Self { Self::new().with_rule(Rule::Root) }

	/// Match elements with the given tag.
	pub fn new() -> Self { Self::default() }

	pub fn with_typed<K: TypedToken, V: TypedToken>(self) -> Self {
		self.with_token(K::key(), V::token())
	}
	pub fn with_value<K: TypedToken>(self, value: impl Typed) -> Result<Self> {
		self.with_token(K::key(), TypedValue::new(value)?).xok()
	}
	/// Add a property mapped to a token.
	pub fn with_token(
		mut self,
		token: TokenKey,
		value: impl Into<TokenValue>,
	) -> Self {
		self.tokens.insert(token, value.into());
		self
	}

	pub fn with_rule(mut self, rule: Rule) -> Self {
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
#[derive(Reflect)]
pub enum Rule {
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
	Not(Vec<Rule>),
}

impl Rule {
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
	pub fn not(rules: Vec<Rule>) -> Self { Self::Not(rules) }

	pub fn matches(&self, el: &ElementView) -> bool {
		match self {
			Rule::Root => true,
			Rule::Tag(tag) => el.element.tag() == tag,
			Rule::Attribute { key, value } => match value {
				Some(expected) => el
					.attribute(key)
					.map(|attr| attr.value == expected)
					.unwrap_or(false),
				None => el.attribute(key).is_some(),
			},
			Rule::State(state) => el.contains_state(state),
			Rule::Class(class) => el.contains_class(class),
			Rule::Not(inner) => !inner.iter().any(|rule| rule.matches(el)),
		}
	}
}
