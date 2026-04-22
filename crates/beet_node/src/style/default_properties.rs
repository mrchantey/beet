use crate::prelude::*;
use crate::style::PropertyMap;
use beet_core::prelude::*;






pub struct DefaultPropertySet {
	// tags to include
	include_tags: Vec<SmolStr>,
	exclude_tags: Vec<SmolStr>,
	property_map: PropertyMap,
}

impl DefaultPropertySet {
	pub fn new(tag:impl Into<SmolStr>)->Self{
		Self{
			include_tags: vec![tag.into()],
			exclude_tags: Vec::new()
		}
	}
	pub fn with(
		mut self,
		property: impl Into<Property>,
		value: Token,
	) -> Self {
		self.0.insert(property.into(), value);
		self
	}	
	
	pub fn passes(&self, el: &Element) -> bool {
		(self.include_tags.is_empty()
			|| self.include_tags.iter().any(|tag| tag == el.tag()))
			&& !self.exclude_tags.iter().any(|tag| tag == el.tag())
	}
}


pub struct DefaultPropertyMap(Vec<DefaultPropertySet>);


pub fn baseline_default_properties()->DefaultPropertyMap{
	DefaultPropertyMap(
	vec![
		DefaultPropertySet::new("a")
			.with(props::FOREGROUND_COLOR, colors::ON_PRIMARY)		
	])
}