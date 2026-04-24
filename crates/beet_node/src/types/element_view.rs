use crate::prelude::*;
use beet_core::prelude::*;

/// Read-only view of an element and its attributes, provided to
/// [`NodeVisitor::visit_element`] for convenient attribute lookup.
pub struct ElementView<'a> {
	/// The entity of this element.
	pub entity: Entity,
	/// The element component.
	pub element: &'a Element,
	/// Attributes for this element.
	pub attributes: Vec<AttributeView<'a>>,
	pub state: Option<&'a ElementStateMap>,
}


pub enum TypedElementViewEnum<'a, Custom = ElementView<'a>> {
	OrderedList(OrderedListView<'a>),
	Link(LinkView<'a>),
	Custom(Custom),
}

impl<'a> ElementView<'a> {
	/// Create a new view from an element reference and its attributes.
	pub fn new(
		entity: Entity,
		element: &'a Element,
		attributes: Vec<AttributeView<'a>>,
		state: Option<&'a ElementStateMap>,
	) -> Self {
		Self {
			entity,
			element,
			attributes,
			state,
		}
	}

	pub fn try_as<T: TypedElementView<'a>>(
		self,
	) -> Result<T, FromElementError> {
		if self.element.tag() == T::TAG {
			T::from_element_view_unchecked(self)
		} else {
			Err(FromElementError::tag_mismatch(T::TAG, self.element.tag()))
		}
	}

	/// The tag name of this element, ie `div`, `span`, `p`.
	pub fn tag(&self) -> &str { self.element.tag() }

	/// Look up the first attribute matching `key` and return its value.
	pub fn attribute<'b>(&'b self, key: &str) -> Option<&'b AttributeView<'a>> {
		self.attributes
			.iter()
			.find(|attr| attr.attribute.as_str() == key)
	}

	pub fn contains_state(&self, state: &ElementState) -> bool {
		self.state.map(|s| s.contains(state)).unwrap_or(false)
	}

	pub fn contains_class(&self, class: &str) -> bool {
		self.attribute("class")
			.map(|attr| {
				attr.value
					.as_str()
					.map(|classes| {
						classes.split_whitespace().any(|c| c == class)
					})
					.unwrap_or(false)
			})
			.unwrap_or(false)
	}

	/// Look up the first attribute matching `key` and return its
	/// `(entity, value)` pair.
	pub fn attribute_with_entity(
		&self,
		key: &str,
	) -> Option<(Entity, &'a Value)> {
		self.attributes
			.iter()
			.find(|attr| attr.attribute.as_str() == key)
			.map(|attr| (attr.entity, attr.value))
	}

	/// Look up an attribute and convert its value to a [`String`].
	/// Returns an empty string when the attribute is absent.
	pub fn attribute_string(&self, key: &str) -> String {
		self.attribute(key)
			.map(|attr| attr.value.to_string())
			.unwrap_or_default()
	}
}

pub trait TypedElementView<'a>: Sized {
	const TAG: &'static str;
	fn from_element_view_unchecked(
		el: ElementView<'a>,
	) -> Result<Self, FromElementError>;
}

pub struct OrderedListView<'a> {
	pub element: ElementView<'a>,
	/// Extract the `start` attribute as a `usize` for ordered lists.
	/// Defaults to `1` when absent or not numeric.
	pub start: usize,
}

impl<'a> TypedElementView<'a> for OrderedListView<'a> {
	/// The HTML tag this view corresponds to, used for type checking
	/// by [`ElementView::try_as`]
	const TAG: &'static str = "ol";
	fn from_element_view_unchecked(
		element: ElementView<'a>,
	) -> Result<Self, FromElementError> {
		let start = element
			.attribute("start")
			.and_then(|attr| match attr.value {
				Value::Uint(num) => Some(*num as usize),
				Value::Int(num) => Some(*num as usize),
				_ => None,
			})
			.unwrap_or(1);
		Ok(Self { element, start })
	}
}

#[derive(Debug, thiserror::Error)]
pub enum FromElementError {
	#[error("expected element with tag '{expected}', but found '{found}'")]
	TagMismatch {
		expected: &'static str,
		found: String,
	},
}

impl FromElementError {
	pub fn tag_mismatch(expected: &'static str, found: &str) -> Self {
		Self::TagMismatch {
			expected,
			found: found.to_string(),
		}
	}
}

pub struct LinkView<'a> {
	pub element: ElementView<'a>,
	pub href: &'a str,
}

impl<'a> TypedElementView<'a> for LinkView<'a> {
	const TAG: &'static str = "a";
	fn from_element_view_unchecked(
		element: ElementView<'a>,
	) -> Result<Self, FromElementError> {
		let href = element
			.attribute("href")
			.and_then(|attr| attr.value.as_str().ok())
			.unwrap_or("");

		Ok(Self { element, href })
	}
}



pub struct AttributeView<'a> {
	/// The entity of this attribute.
	pub entity: Entity,
	/// The attribute component.
	pub attribute: &'a Attribute,
	/// The value for this attribute.
	pub value: &'a Value,
}


impl<'a> AttributeView<'a> {
	pub fn key(&self) -> &str { &self.attribute }
}
