//! Descendant text content of an element subtree.
//!
//! The raw-text body of an element (eg a `<script>` or `<style>`) is its
//! [`Value::Str`] descendants concatenated in order. [`ElementTextQuery`] is the
//! single accessor for that operation, the descendant counterpart to
//! [`ElementTraverseQuery`]'s ancestry walk, and for a script's source, the
//! same walk stopping at a nested [`RunScript`]. The inverse,
//! [`Element::with_inner_text`], sets it.

use crate::prelude::*;

/// Read access to the text content of an element subtree.
///
/// [`text_content`](Self::text_content) concatenates the [`Value::Str`] text of an
/// entity and its descendants in pre-order: the verbatim body of a raw-text
/// element, for any render path that needs an element's text content.
/// [`script_source`](Self::script_source) is the same walk for a script.
#[derive(SystemParam)]
pub struct ElementTextQuery<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	values: Query<'w, 's, &'static Value>,
	scripts: Query<'w, 's, (), With<RunScript>>,
}

impl ElementTextQuery<'_, '_> {
	/// The concatenated [`Value::Str`] text of `entity` and its descendants in
	/// pre-order, ie the verbatim body of a raw-text `<script>` or `<style>`.
	pub fn text_content(&self, entity: Entity) -> String {
		let mut out = String::new();
		self.push_text(entity, &mut out);
		out
	}

	/// The program of the script `entity`: its own text and every
	/// descendant's in pre-order, stopping at a descendant carrying its own
	/// [`RunScript`], which is a script of its own.
	pub fn script_source(&self, entity: Entity) -> String {
		let mut out = String::new();
		self.push_source(entity, &mut out);
		out
	}

	/// Append `entity`'s own text, then recurse into its children in order.
	fn push_text(&self, entity: Entity, out: &mut String) {
		self.push_own(entity, out);
		for child in self.children.get(entity).into_iter().flatten() {
			self.push_text(*child, out);
		}
	}

	/// [`push_text`](Self::push_text), skipping every nested script.
	fn push_source(&self, entity: Entity, out: &mut String) {
		self.push_own(entity, out);
		for child in self
			.children
			.get(entity)
			.into_iter()
			.flatten()
			.filter(|child| !self.scripts.contains(**child))
		{
			self.push_source(*child, out);
		}
	}

	fn push_own(&self, entity: Entity, out: &mut String) {
		if let Ok(Value::Str(value)) = self.values.get(entity) {
			out.push_str(value.as_str());
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Text content is the element's `Value::Str` descendants concatenated in
	/// pre-order, so a multi-node raw-text body reads back verbatim.
	#[crate::test]
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

	/// A script's source is its own text then its descendants', stopping at
	/// a nested script, which is a script of its own.
	#[crate::test]
	fn a_script_source_stops_at_a_nested_script() {
		let mut world = World::new();
		let root = world
			.spawn((RunScript, Value::Str("a".into()), children![
				Value::Str("b".into()),
				(RunScript, Value::Str("x".into()), children![Value::Str(
					"y".into()
				)]),
				(Element::new("span"), children![Value::Str("c".into())]),
			]))
			.id();
		world
			.run_system_cached_with(
				|entity: In<Entity>, elements: ElementTextQuery| {
					elements.script_source(*entity)
				},
				root,
			)
			.unwrap()
			.xpect_eq("abc".to_string());
	}
}
