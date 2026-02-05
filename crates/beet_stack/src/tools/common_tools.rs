use crate::prelude::*;
use beet_core::prelude::*;


/// A tool that increments a specified field when triggered, returning the new value.
pub fn increment(field: FieldRef) -> impl Bundle {
	let _a = move |cx: In<ToolContext>, mut query: DocumentQuery| -> Result {
		let _doc = query.get_mut(cx.tool, &field.document)?;
		Ok(())
	};
}



#[cfg(test)]
mod test {
	use super::*;

	/// A field reference to a count
	fn count() -> FieldRef { FieldRef::new(DocumentPath::Card, "count") }

	fn counter() -> impl Bundle {}

	#[test]
	fn works() {
		let mut world = World::new();

		let tool = world.spawn(increment(count()));
		
	}
}
