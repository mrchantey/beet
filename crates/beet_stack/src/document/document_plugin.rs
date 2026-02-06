use crate::document::*;
use crate::prelude::*;
use beet_core::prelude::*;


/// Plugin that enables document synchronization for text content.
///
/// This plugin:
/// - Links [`FieldRef`] components to their associated documents
/// - Automatically updates [`TextContent`] when documents change
///
/// # Example
///
/// ```
/// use beet_stack::prelude::*;
/// use beet_core::prelude::*;
///
/// let mut world = DocumentPlugin::world();
///
/// // Create a card with a document
/// let card = world
///     .spawn((Card, Document::new(val!({ "name": "Alice" }))))
///     .id();
///
/// // Spawn text that references the document field
/// let entity = world.spawn((
///     ChildOf(card),
///     TextContent::default(),
///     FieldRef::new("name"),
/// )).id();
///
/// // After update, TextContent will contain "Alice"
/// world.update_local();
///
/// let value = world.entity(entity).get::<TextContent>().unwrap().as_str();
/// assert_eq!(value, "Alice");
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
			// Register content types
			.register_type::<TextContent>()
			.register_type::<TextBlock>()
			.register_type::<Title>()
			.register_type::<Paragraph>()
			.register_type::<Important>()
			.register_type::<Emphasize>()
			.register_type::<Code>()
			.register_type::<Quote>()
			.register_type::<Link>()
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

		// Create card with document
		let card = world
			.spawn((Card, Document::new(val!({ "greeting": "Hello" }))))
			.id();

		// Spawn text content with field ref as child
		world.spawn((
			ChildOf(card),
			TextContent::default(),
			FieldRef::new("greeting"),
		));

		// Run update to trigger sync
		world.update_local();

		// Verify text was updated
		let text = world.query_once::<&TextContent>()[0].clone();
		text.0.xpect_eq("Hello");
	}

	#[test]
	fn text_field_syncs_on_document_change() {
		let mut world = DocumentPlugin::world();

		// Create card with document
		let card = world
			.spawn((Card, Document::new(val!({ "count": 0i64 }))))
			.id();

		// Spawn text content with field ref
		world.spawn((
			ChildOf(card),
			TextContent::default(),
			FieldRef::new("count"),
		));

		world.update_local();

		// Initial value
		let text = world.query_once::<&TextContent>()[0].clone();
		text.0.xpect_eq("0");

		// Update the document
		world.entity_mut(card).get_mut::<Document>().unwrap().0 =
			val!({ "count": 42i64 });

		world.update_local();

		// Value should be updated
		let text = world.query_once::<&TextContent>()[0].clone();
		text.0.xpect_eq("42");
	}

	#[test]
	fn text_field_with_nested_path() {
		let mut world = DocumentPlugin::world();

		let card = world
			.spawn((Card, Document::new(val!({ "user": { "name": "Bob" } }))))
			.id();

		world.spawn((
			ChildOf(card),
			TextContent::default(),
			FieldRef::new(vec!["user", "name"]),
		));

		world.update_local();

		let text = world.query_once::<&TextContent>()[0].clone();
		text.0.xpect_eq("Bob");
	}

	#[test]
	fn multiple_text_fields_same_document() {
		let mut world = DocumentPlugin::world();

		let card = world
			.spawn((
				Card,
				Document::new(val!({
					"first": "Alice",
					"second": "Bob"
				})),
			))
			.id();

		world.spawn((
			ChildOf(card),
			TextContent::default(),
			FieldRef::new("first"),
		));

		world.spawn((
			ChildOf(card),
			TextContent::default(),
			FieldRef::new("second"),
		));

		world.update_local();

		let texts: Vec<_> = world
			.query_once::<&TextContent>()
			.iter()
			.map(|t| t.0.clone())
			.collect();

		texts.contains(&"Alice".to_string()).xpect_true();
		texts.contains(&"Bob".to_string()).xpect_true();
	}

	#[test]
	fn text_block_with_field_ref() {
		let mut world = DocumentPlugin::world();

		let card = world
			.spawn((Card, Document::new(val!({ "name": "World" }))))
			.id();

		world.spawn((ChildOf(card), text!["Hello, ", FieldRef::new("name")]));

		world.update_local();

		// Find the text content with field ref
		let texts: Vec<_> = world
			.query_once::<(&TextContent, Option<&FieldRef>)>()
			.iter()
			.map(|(t, f)| (t.0.clone(), f.is_some()))
			.collect();

		// One should be static "Hello, ", one should be dynamic "World"
		texts
			.iter()
			.any(|(t, has_field)| t == "Hello, " && !has_field)
			.xpect_true();
		texts
			.iter()
			.any(|(t, has_field)| t == "World" && *has_field)
			.xpect_true();
	}
}
