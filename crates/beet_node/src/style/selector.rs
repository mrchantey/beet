use crate::prelude::*;
use beet_core::prelude::*;

/// A set of default properties applied to elements matching the given criteria.
#[derive(Default, Get)]
pub struct Selector {
	/// All the rules an element must match for styles to be applied.
	/// Empty matches all elements
	rules: Vec<Rule>,
	tokens: TokenMap2,
}

// akin to a lightningcss Component, ie `/selectors/parser.rs#1392`
/// A match rule
pub enum Rule {
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
	Not(Box<Rule>),
}

impl Rule {
	pub fn matches(&self, el: &ElementView) -> bool {
		match self {
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
			Rule::Not(inner) => !inner.matches(el),
		}
	}
}


impl Selector {
	/// Match elements with the given tag.
	pub fn new() -> Self { Self::default() }

	/// Add a property mapped to a token.
	pub fn with_token(
		mut self,
		path: FieldPath,
		value: Token2,
	) -> Result<Self> {
		self.tokens.insert(path, value)?;
		self.xok()
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
