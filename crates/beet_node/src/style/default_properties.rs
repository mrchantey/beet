use crate::prelude::*;
use crate::style::PropertyMap;
use beet_core::prelude::*;






pub struct DefaultPropertySet<T> {
	// tags to include
	include_tags: Vec<SmolStr>,
	exclude_tags: Vec<SmolStr>,
	_property_map: PropertyMap<T>,
}

impl<T> DefaultPropertySet<T> {
	/// Checks if a path passes the filter.
	///
	/// To pass a path must:
	/// 1. Not be present in the exclude patterns
	/// 2. Be present in the include patterns or the include patterns are empty
	///
	/// Currently converts paths to strings with forward slashes.
	pub fn passes(&self, el: &Element) -> bool {
		(self.include_tags.is_empty()
			|| self.include_tags.iter().any(|tag| tag == el.tag()))
			&& !self.exclude_tags.iter().any(|tag| tag == el.tag())
	}
}
