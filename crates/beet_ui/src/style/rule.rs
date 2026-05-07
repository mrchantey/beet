use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::sync::Arc;

/// A set of default properties applied to elements matching the given criteria.
#[derive(Debug, Default, Clone, Reflect, Get, SetWith)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rule {
	predicate: Predicate,
	declarations: TokenStore,
}


impl Rule {
	pub fn new() -> Self { Self::default() }

	pub fn insert(
		&mut self,
		key: impl Into<Token>,
		value: impl Into<TokenValue>,
	) -> Result<&mut Self> {
		self.declarations.insert(key, value)?;
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
	pub fn merge_any(mut self, other: Self) -> Self {
		self.predicate = self.predicate.clone().merge_any(other.predicate);
		self.declarations = self.declarations.extend(other.declarations);
		self
	}


	/// Matches all rules, or `true` if empty
	pub fn matches(&self, el: &ElementView) -> bool {
		self.predicate.matches(el)
	}
}

// akin to a lightningcss Component, ie `/selectors/parser.rs#1392`
/// A match rule
#[derive(Debug, Default, Clone, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Predicate {
	/// A global predicate, in css this will evaluate to `:root`,
	/// and in bevy apps will always pass predicates
	#[default]
	Root,
	/// Selects any element, in css this will evaluate to `*`,
	/// and in bevy apps will always pass predicates
	Any,
	/// Match any of the rules, eg `div, .my-class` (note the comma) in css
	AnyOf(Vec<Predicate>),
	/// Match all of the rules, eg `div.my-class` (note no comma) in css
	AllOf(Vec<Predicate>),
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

impl Predicate {
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
	pub fn not(inner: Predicate) -> Self { Self::Not(Arc::new(inner)) }

	/// Merge two rules, as an AnyOf,
	/// collapsing global selectors like Root and Any
	pub fn merge_any(self, other: Self) -> Self {
		match (self, other) {
			(Self::Root, Self::Root) => Self::Root,
			// (Self::Root, r) | (r, Self::Root) => r,
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
			Predicate::Root => true,
			Predicate::Any => true,
			Predicate::AnyOf(rules) => {
				rules.iter().any(|rule| rule.matches(el))
			}
			Predicate::AllOf(rules) => {
				rules.iter().all(|rule| rule.matches(el))
			}
			Predicate::Tag(tag) => el.element.tag() == tag,
			Predicate::Attribute { key, value } => match value {
				Some(expected) => el
					.attribute(key)
					.map(|attr| attr.value == expected)
					.unwrap_or(false),
				None => el.attribute(key).is_some(),
			},
			Predicate::State(state) => el.contains_state(state),
			Predicate::Class(class) => el.contains_class(class),
			Predicate::Not(inner) => !inner.matches(el),
		}
	}
}
