//! Descendant text content of an element subtree.
//!
//! The raw-text body of an element (eg a `<script>` or `<style>`) is its
//! [`Value::Str`] descendants concatenated in order. [`ElementTextQuery`] is the
//! single accessor for that operation, the descendant counterpart to
//! [`ElementTraverseQuery`]'s ancestry walk. The inverse,
//! [`Element::with_inner_text`], sets it.

use crate::prelude::*;

/// Read access to the text content of an element subtree.
///
/// [`text_content`](Self::text_content) concatenates the [`Value::Str`] text of an
/// entity and its descendants in pre-order: the verbatim body of a raw-text
/// element, shared by the `RunScript` script verb and any render path that needs
/// an element's text content.
#[derive(SystemParam)]
pub struct ElementTextQuery<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	values: Query<'w, 's, &'static Value>,
}

impl ElementTextQuery<'_, '_> {
	/// The concatenated [`Value::Str`] text of `entity` and its descendants in
	/// pre-order, ie the verbatim body of a raw-text `<script>` or `<style>`.
	pub fn text_content(&self, entity: Entity) -> String {
		let mut out = String::new();
		self.push_text(entity, &mut out);
		out
	}

	/// Append `entity`'s own text, then recurse into its children in order.
	fn push_text(&self, entity: Entity, out: &mut String) {
		if let Ok(Value::Str(value)) = self.values.get(entity) {
			out.push_str(value.as_str());
		}
		if let Ok(children) = self.children.get(entity) {
			children.iter().for_each(|child| self.push_text(child, out));
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Text content is the element's `Value::Str` descendants concatenated in
	/// pre-order, so a multi-node raw-text body reads back verbatim.
	#[beet_core::test]
	fn concatenates_descendant_text() {
		let mut world = World::new();
		let root = world
			.spawn((Element::new("script"), children![
				Value::Str("console.log(".into()),
				Value::Str("\"hi\")".into()),
			]))
			.id();
		world
			.run_system_cached_with(
				|entity: In<Entity>, elements: ElementTextQuery| {
					elements.text_content(*entity)
				},
				root,
			)
			.unwrap()
			.xpect_eq("console.log(\"hi\")".to_string());
	}
}
