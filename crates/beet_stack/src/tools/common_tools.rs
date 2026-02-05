use crate::prelude::*;
use beet_core::prelude::*;


/// A tool that increments a specified field when triggered, returning the new value.
pub fn increment(field: FieldRef) -> impl Bundle {
	let _a = move |cx: In<ToolContext>, mut query: DocumentQuery| -> Result {
		let _doc = query.get_mut(cx.tool, &field.document)?;
		Ok(())
	};
}
