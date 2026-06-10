use super::*;
use crate::prelude::*;


/// Plugin that enables document synchronization for field values.
///
/// This plugin:
/// - Links [`FieldRef`] components to their associated documents
/// - Automatically updates [`Value`] when documents change (read path)
/// - Writes a changed [`Value`] back into its document field (write-back)
///
/// # Example
///
/// ```
/// use beet_core::prelude::*;
///
/// let mut world = DocumentPlugin::world();
///
/// // Create a document with a value child
/// world.spawn((
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
			.register_type::<DocumentSchema>()
			.register_type::<DocumentPath>()
			.register_type::<OnMissingField>()
			.register_type::<FieldRef>()
			.register_type::<FieldSegment>()
			.register_type::<DocumentScope>()
			.register_type::<ResolvedFieldPath>()
			.register_type::<Value>()
			.register_type::<ValueSchema>();

		app
			// Add observers and systems
			.add_observer(link_field_to_document)
			.add_observer(unlink_field_from_document)
			.add_observer(resolve_field_path);

		// the document sync chain. With `bevy_async` it runs after the async sync
		// point so an async field write (eg refresh_blob_store_list) lands the
		// same pass; without it (no_std core) there is no sync point to order on.
		#[cfg(feature = "json")]
		app.register_type::<ReflectFieldRef>();

		// the reflect-field-binding sync mirrors a `Value` to/from a reflected
		// component field, the generalization of the `Value`-component bind. It
		// runs after the document drives `Value` so the read direction lands first.
		#[cfg(feature = "json")]
		let reflect_sync = sync_reflect_field_bindings;
		#[cfg(not(feature = "json"))]
		let reflect_sync = || {};

		let sync_chain = (
			update_resolved_field_paths.run_if(resolved_paths_need_update),
			sync_schema.run_if(schema_needs_sync),
			sync_document_to_local,
			sync_resolved_path_changes,
			// reflect-field binding runs between the read path and the write-back,
			// so a document change reaches the component and a component edit reaches
			// the document, both within one pass.
			reflect_sync,
			sync_local_to_document,
			update_reactive_children,
		)
			.chain();
		#[cfg(feature = "bevy_async")]
		app.add_systems(
			PreUpdate,
			sync_chain.after(async_world_sync_point::<BeetAsyncSyncPoint>),
		);
		#[cfg(not(feature = "bevy_async"))]
		app.add_systems(PreUpdate, sync_chain);
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn text_field_syncs_on_insert() {
		let mut world = DocumentPlugin::world();

		// Create document with a value child
		world.spawn((Document::new(val!({ "greeting": "Hello" })), children![
			(Value::default(), FieldRef::new("greeting"))
		]));

		// Run update to trigger sync
		world.update_local();

		// Verify value was updated
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Str("Hello".into()));
	}

	#[beet_core::test]
	fn text_field_syncs_on_document_change() {
		let mut world = DocumentPlugin::world();

		// Create document with a value child
		let card = world
			.spawn((Document::new(val!({ "count": 0i64 })), children![(
				Value::default(),
				FieldRef::new("count")
			)]))
			.id();

		world.update_local();

		// Initial value
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Int(0));

		// Update the document
		world.entity_mut(card).get_mut::<Document>().unwrap().0 =
			val!({ "count": 42i64 });

		world.update_local();

		// Value should be updated
		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Int(42));
	}

	#[beet_core::test]
	fn text_field_with_nested_path() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			Document::new(val!({ "user": { "name": "Bob" } })),
			children![(Value::default(), FieldRef::new(vec!["user", "name"]))],
		));

		world.update_local();

		let (value, _) = &world.query_once::<(&Value, &FieldRef)>()[0];
		(*value).clone().xpect_eq(Value::Str("Bob".into()));
	}

	#[beet_core::test]
	fn multiple_text_fields_same_document() {
		let mut world = DocumentPlugin::world();

		world.spawn((
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

	#[beet_core::test]
	fn text_block_with_field_ref() {
		let mut world = DocumentPlugin::world();

		world.spawn((Document::new(val!({ "name": "World" })), children![
			Value::Str("Hello, ".into()),
			(FieldRef::new("name"), Value::default()),
		]));

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
