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
/// // Create a card with a document and text child
/// world.spawn((
///     Card,
///     Document::new(val!({ "name": "Alice" })),
///     children![(TextContent::default(), FieldRef::new("name"))],
/// ));
///
/// // After update, TextContent will contain "Alice"
/// world.update_local();
///
/// let value = world.query_once::<&TextContent>()[0].as_str();
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
			.register_type::<Title>()
			.register_type::<Paragraph>()
			.register_type::<Important>()
			.register_type::<Emphasize>()
			.register_type::<Code>()
			.register_type::<Quote>()
			.register_type::<Link>()
			// Register layout types
			.register_type::<TitleLevel>()
			// Add observers and systems
			.add_observer(link_field_to_document)
			.add_observer(unlink_field_from_document)
			.add_observer(crate::content::calculate_title_level)
			.add_systems(PreUpdate, update_text_fields);
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn text_field_syncs_on_insert() {
		let mut world = DocumentPlugin::world();

		// Create card with document and text child
		world.spawn((
			Card,
			Document::new(val!({ "greeting": "Hello" })),
			children![(TextContent::default(), FieldRef::new("greeting"))],
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

		// Create card with document and text child
		let card = world
			.spawn((Card, Document::new(val!({ "count": 0i64 })), children![(
				TextContent::default(),
				FieldRef::new("count")
			)]))
			.id();

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

		world.spawn((
			Card,
			Document::new(val!({ "user": { "name": "Bob" } })),
			children![(
				TextContent::default(),
				FieldRef::new(vec!["user", "name"])
			)],
		));

		world.update_local();

		let text = world.query_once::<&TextContent>()[0].clone();
		text.0.xpect_eq("Bob");
	}

	#[test]
	fn multiple_text_fields_same_document() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			Card,
			Document::new(val!({
				"first": "Alice",
				"second": "Bob"
			})),
			children![
				(TextContent::default(), FieldRef::new("first")),
				(TextContent::default(), FieldRef::new("second"))
			],
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

		world.spawn((
			Card,
			Document::new(val!({ "name": "World" })),
			children![(content!["Hello, ", FieldRef::new("name")])],
		));

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
