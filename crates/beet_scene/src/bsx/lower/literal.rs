//! Leaf lowering: a tag's attributes to the value they hold.
use super::lowering::*;
use beet_core::prelude::*;

impl Lowering<'_> {
	/// A tag's literal attributes as the fields of the component it names. A
	/// spread and an `@` binding are not fields, each applying to the entity
	/// through its own machinery.
	pub fn attributes_to_held(&self, el: &BsxElement) -> Value {
		el.plain_attributes()
			.filter_map(|attr| {
				attr.value.literal().ok().map(|literal| {
					(
						SmolStr::from(attr.key.as_str()),
						self.values().literal_to_held(&literal),
					)
				})
			})
			.collect::<Map>()
			.xmap(Value::Map)
	}
}
