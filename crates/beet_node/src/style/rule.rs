use std::sync::Arc;

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// A set of default properties applied to elements matching the given criteria.
#[derive(Debug, Default, Clone, Reflect, Get, SetWith)]
pub struct Rule {
	selector: Selector,
	declarations: HashMap<TokenKey, TokenValue>,
}


impl Rule {
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


	/// Matches all rules, or `true` if empty
	pub fn matches(&self, el: &ElementView) -> bool {
		self.selector.matches(el)
	}
}

// akin to a lightningcss Component, ie `/selectors/parser.rs#1392`
/// A match rule
#[derive(Debug, Default, Clone, Reflect)]
pub enum Selector {
	/// A global selector, in css this will evaluate to `:root`,
	/// and in bevy apps will always pass predicates
	#[default]
	Root,
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

	/// Merge two rules, as an AnyOf,
	/// collapsing global selectors like Root and Any
	pub fn merge_any(self, other: Self) -> Self {
		match (self, other) {
			(Self::Root, Self::Root) => Self::Root,
			// (Self::Root, r) | (r, Self::Root) => r,
			(Self::Any, _) | (_, Self::Any) => Self::Any,
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
