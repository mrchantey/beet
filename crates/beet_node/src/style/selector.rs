use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Clone, Reflect, Get)]
pub struct Property2 {
	/// The name of this property in css,
	/// ie `background-color`
	css_name: SmolStr,
	/// Whether this property should traverse
	/// up the stack and inherit parent properties
	inherit_base: bool,
	/// Token for the value of this property.
	value: Token2,
}

todo!("replace these with rules, ie negated etc");
/// A set of default properties applied to elements matching the given criteria.
#[derive(Get)]
pub struct Selector {
	/// Element tags to match; empty means match any tag.
	include_tags: Vec<SmolStr>,
	exclude_tags: Vec<SmolStr>,
	/// Attribute keys to match, with optional value constraint.
	include_attributes: Vec<(SmolStr, Option<Value>)>,
	exclude_attributes: Vec<(SmolStr, Option<Value>)>,
	tokens: TokenMap2,
}

// akin to a lightningcss Component, ie `/selectors/parser.rs#1392`
pub enum Rule {}

impl Selector {
	/// Match elements with the given tag.
	pub fn new_tag(tag: impl Into<SmolStr>) -> Self {
		Self {
			include_tags: vec![tag.into()],
			exclude_tags: Vec::new(),
			include_attributes: Vec::new(),
			exclude_attributes: Vec::new(),
			tokens: default(),
		}
	}

	/// Match any element regardless of tag.
	pub fn any() -> Self {
		Self {
			include_tags: Vec::new(),
			exclude_tags: Vec::new(),
			include_attributes: Vec::new(),
			exclude_attributes: Vec::new(),
			tokens: default(),
		}
	}

	/// Add a property mapped to a token.
	pub fn with(mut self, path: FieldPath, value: Token2) -> Result<Self> {
		self.tokens.insert(path, value)?;
		self.xok()
	}

	/// Returns true if the element satisfies all tag and attribute criteria.
	pub fn passes(&self, el: &ElementView) -> bool {
		self.passes_tags(el.element) && self.passes_attributes(el)
	}

	pub fn passes_tags(&self, el: &Element) -> bool {
		(self.include_tags.is_empty()
			|| self.include_tags.iter().any(|tag| tag == el.tag()))
			&& !self.exclude_tags.iter().any(|tag| tag == el.tag())
	}

	pub fn passes_attributes(&self, el: &ElementView) -> bool {
		(self.include_attributes.is_empty()
			|| self.include_attributes.iter().any(|(key, val)| match val {
				Some(expected) => {
					el.attribute(key).map(|v| v == expected).unwrap_or(false)
				}
				None => el.attribute(key).is_some(),
			})) && !self.exclude_attributes.iter().any(|(key, val)| match val {
			Some(expected) => {
				el.attribute(key).map(|v| v == expected).unwrap_or(false)
			}
			None => el.attribute(key).is_some(),
		})
	}
}
