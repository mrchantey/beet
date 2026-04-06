#[cfg(feature = "tool")]
use super::common_tools;
use crate::document::*;
use crate::prelude::*;
use beet_core::prelude::*;


/// Plugin that enables document synchronization for field values.
///
/// This plugin:
/// - Links [`FieldRef`] components to their associated documents
/// - Automatically updates [`Value`] when documents change
///
/// # Example
///
/// ```
/// use beet_node::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = DocumentPlugin::world();
///
/// // Create a scope with a document and value child
/// world.spawn((
///     DocumentScope,
///     Document::new(val!({ "name": "Alice" })),
///     children![(Value::default(), FieldRef::new("name"))],
/// ));
///
/// // After update, Value will contain Str("Alice")
/// world.update_local();
///
/// let values = world.query_once::<(&Value, &FieldRef)>();
/// let (value, _) = &values[0];
/// assert_eq!(*value, &Value::Str("Alice".into()));
/// ```
#[derive(Default)]
pub struct DocumentPlugin;

impl Plugin for DocumentPlugin {
	fn build(&self, app: &mut App) {
		app
			// Register document types
			.register_type::<Document>()
			.register_type::<DocumentPath>()
			.register_type::<OnMissingField>()
			.register_type::<FieldRef>()
			.register_type::<FieldPath>()
			.register_type::<Value>()
			.register_type::<DocumentScope>();

		// Register tool types when the tool feature is enabled
		#[cfg(feature = "tool")]
		app.register_type::<common_tools::Increment>()
			.register_type::<common_tools::Decrement>()
			.register_type::<common_tools::AddField>()
			.register_type::<common_tools::SetField>()
			.register_type::<common_tools::ReadField>();

		app
			// Add observers and systems
			.add_observer(link_field_to_document)
			.add_observer(unlink_field_from_document)
			.add_systems(PreUpdate, update_text_fields);
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn text_field_syncs_on_insert() {
		let mut world = DocumentPlugin::world();

		// Create scope with document and value child
		world.spawn((
			DocumentScope,
			Document::new(val!({ "greeting": "Hello" })),
			children![(Value::default(), FieldRef::new("greeting"))],
		));

		// Run update to trigger sync
		world.update_local();

		// Verify value was updated
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Str("Hello".into()));
	}

	#[test]
	fn text_field_syncs_on_document_change() {
		let mut world = DocumentPlugin::world();

		// Create scope with document and value child
		let card = world
			.spawn((
				DocumentScope,
				Document::new(val!({ "count": 0i64 })),
				children![(Value::default(), FieldRef::new("count"))],
			))
			.id();

		world.update_local();

		// Initial value
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Str("0".into()));

		// Update the document
		world.entity_mut(card).get_mut::<Document>().unwrap().0 =
			val!({ "count": 42i64 });

		world.update_local();

		// Value should be updated
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Str("42".into()));
	}

	#[test]
	fn text_field_with_nested_path() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			DocumentScope,
			Document::new(val!({ "user": { "name": "Bob" } })),
			children![(Value::default(), FieldRef::new(vec!["user", "name"]))],
		));

		world.update_local();

		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Str("Bob".into()));
	}

	#[test]
	fn multiple_text_fields_same_document() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			DocumentScope,
			Document::new(val!({
				"first": "Alice",
				"second": "Bob"
			})),
			children![
				(Value::default(), FieldRef::new("first")),
				(Value::default(), FieldRef::new("second"))
			],
		));

		world.update_local();

		let results: Vec<_> = world
			.query_once::<(&Value, &FieldRef)>()
			.iter()
			.map(|(value, _)| (*value).clone())
			.collect();

		results.contains(&Value::Str("Alice".into())).xpect_true();
		results.contains(&Value::Str("Bob".into())).xpect_true();
	}

	#[test]
	fn text_block_with_field_ref() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			DocumentScope,
			Document::new(val!({ "name": "World" })),
			children![
				Value::Str("Hello, ".into()),
				(FieldRef::new("name"), Value::default()),
			],
		));

		world.update_local();

		// Find the value content with field ref
		let results: Vec<_> = world
			.query_once::<(&Value, Option<&FieldRef>)>()
			.iter()
			.map(|(value, field)| ((*value).clone(), field.is_some()))
			.collect();

		// One should be static "Hello, ", one should be dynamic "World"
		results
			.iter()
			.any(|(value, has_field)| {
				*value == Value::Str("Hello, ".into()) && !has_field
			})
			.xpect_true();
		results
			.iter()
			.any(|(value, has_field)| {
				*value == Value::Str("World".into()) && *has_field
			})
			.xpect_true();
	}
}
