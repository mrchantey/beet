use crate::prelude::*;
use crate::style::PropertyMap;
use beet_core::prelude::*;

pub struct DefaultPropertySet {
	/// Element tags to match
	include_tags: Vec<SmolStr>,
	exclude_tags: Vec<SmolStr>,
	/// Attribute keys to match,
	/// and optionally also ensure values match
	include_attributes: Vec<SmolStr, Option<Value>>,
	exclude_attribute: Vec<SmolStr, Option<Value>>,
	property_map: PropertyMap,
}

impl DefaultPropertySet {
	pub fn new(tag: impl Into<SmolStr>) -> Self {
		Self {
			include_tags: vec![tag.into()],
			exclude_tags: Vec::new(),
			property_map: PropertyMap::default(),
		}
	}

	pub fn with(
		mut self,
		property: impl Into<Property>,
		value: impl Into<Token>,
	) -> Self {
		self.property_map.insert(property.into(), value.into());
		self
	}

	pub fn passes(&self, el: &ElementView) -> bool {}

	pub fn passes_tags(&self, el: &Element) -> bool {
		(self.include_tags.is_empty()
			|| self.include_tags.iter().any(|tag| tag == el.tag()))
			&& !self.exclude_tags.iter().any(|tag| tag == el.tag())
	}

	pub fn passes_attributes(&self, el: &ElementView) -> bool {
		(self.include_attributes.is_empty()
			|| self.include_attributes.iter().any(|(key, val)| {
				// complete the check
			})) && !self.exclude_attributes.iter().any(|(key, val)| {
			// complete here
		})
	}

	pub fn property_map(&self) -> &PropertyMap { &self.property_map }
}

#[derive(Resource, Component)]
pub struct DefaultPropertyMap(Vec<DefaultPropertySet>);

impl DefaultPropertyMap {
	pub fn new(sets: Vec<DefaultPropertySet>) -> Self { Self(sets) }

	pub fn iter(&self) -> impl Iterator<Item = &DefaultPropertySet> {
		self.0.iter()
	}
}

pub fn baseline_default_properties() -> DefaultPropertyMap {
	DefaultPropertyMap::new(vec![
		DefaultPropertySet::new("a")
			.with(&*props::FOREGROUND_COLOR, &*colors::ON_PRIMARY),
	])
}
