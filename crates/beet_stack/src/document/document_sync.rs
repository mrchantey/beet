//! Document synchronization for text content fields.
//!
//! This module provides the machinery for keeping [`TextNode`] components
//! in sync with their associated [`Document`] fields through [`FieldRef`].
//!
//! # Architecture
//!
//! The synchronization works through a relationship system:
//!
//! 1. When a [`FieldRef`] is inserted, an observer creates a [`FieldOf`]
//!    relationship pointing to the resolved document entity.
//!
//! 2. The [`Fields`] component on document entities tracks all field
//!    references that depend on it.
//!
//! 3. When a [`Document`] changes, the `update_text_fields` system iterates
//!    through all related [`FieldRef`] entities and updates their [`TextNode`].
//!
//! # Example
//!
//! ```ignore
//! use beet_stack::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = DocumentPlugin::world();
//!
//! // Create a card with a document and text child
//! world.spawn((
//!     Card,
//!     Document::new(val!({ "score": 100i64 })),
//!     children![(TextNode::default(), FieldRef::new("score"))],
//! ));
//!
//! // After update, TextNode contains "100"
//! world.update_local();
//! ```

use crate::prelude::*;
use beet_core::prelude::*;



/// Tracks every [`FieldRef`] associated with this [`Document`] entity.
///
/// This component is automatically managed through Bevy's relationship system.
/// The entity may or may not have been initialized with a [`Document`] -
/// the relationship is established based on [`DocumentPath`] resolution.
#[derive(Component)]
#[relationship_target(relationship = FieldOf)]
pub struct Fields(Vec<Entity>);

/// Attached to a [`FieldRef`] to track its associated [`Document`] entity.
///
/// This relationship is created when a [`FieldRef`] is inserted and allows
/// the document to find all text fields that depend on it for updates.
///
/// As [`FieldRef`] is immutable, this relationship is only added on insert
/// and removed when the [`FieldRef`] is removed.
#[derive(Component)]
#[relationship(relationship_target = Fields)]
pub struct FieldOf {
	/// The document entity this field references.
	#[relationship]
	pub document: Entity,
}

/// Observer that creates the [`FieldOf`] relationship when a [`FieldRef`] is inserted.
///
/// Resolves the [`DocumentPath`] to find the actual document entity and
/// establishes the relationship so document changes can efficiently propagate to this field.
pub(super) fn link_field_to_document(
	ev: On<Insert, FieldRef>,
	mut commands: Commands,
	fields: Query<&FieldRef>,
	mut doc_query: DocumentQuery,
) -> Result {
	let field = fields.get(ev.entity)?;
	let document = doc_query.entity(ev.entity, &field.document);
	commands.entity(ev.entity).insert(FieldOf { document });
	Ok(())
}

/// Observer that removes the [`FieldOf`] relationship when a [`FieldRef`] is removed.
pub(super) fn unlink_field_from_document(
	ev: On<Remove, FieldRef>,
	mut commands: Commands,
) -> Result {
	commands.entity(ev.entity).remove::<FieldOf>();
	Ok(())
}


/// System that updates [`TextNode`] when their associated [`Document`] changes.
///
/// Runs in `PreUpdate` to ensure text is synchronized before user systems run.
pub(super) fn update_text_fields(
	query: Populated<(&Document, &Fields), Changed<Document>>,
	mut text_fields: Query<(&FieldRef, &mut TextNode)>,
) -> Result {
	for (doc, doc_fields) in query {
		for field in doc_fields.iter() {
			if let Ok((field_ref, mut text)) = text_fields.get_mut(field) {
				let field = doc.get_field_ref(&field_ref.field_path)?;
				text.0 = field.to_string();
			}
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn link_creates_relationship() {
		let mut world = DocumentPlugin::world();

		let card = world
			.spawn((Card, Document::new(val!({ "x": "value" }))))
			.id();
		let text = world
			.spawn((ChildOf(card), TextNode::default(), FieldRef::new("x")))
			.id();

		world.update_local();

		// FieldOf should point to the card
		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(card);

		// Card should have Fields tracking the text entity
		let fields = world.entity(card).get::<Fields>().unwrap();
		fields
			.iter()
			.collect::<Vec<_>>()
			.contains(&text)
			.xpect_true();
	}

	#[test]
	fn unlink_removes_relationship() {
		let mut world = DocumentPlugin::world();

		let card = world
			.spawn((Card, Document::new(val!({ "x": "value" }))))
			.id();
		let text = world
			.spawn((ChildOf(card), TextNode::default(), FieldRef::new("x")))
			.id();

		world.update_local();

		// Verify relationship exists
		world.entity(text).contains::<FieldOf>().xpect_true();

		// Remove the FieldRef
		world.entity_mut(text).remove::<FieldRef>();
		world.update_local();

		// FieldOf should be gone
		world.entity(text).contains::<FieldOf>().xpect_false();
	}

	#[test]
	fn resolves_root_document_path() {
		let mut world = DocumentPlugin::world();

		let root = world
			.spawn(Document::new(val!({ "root_val": "from_root" })))
			.id();
		let child = world.spawn(ChildOf(root)).id();
		let text = world
			.spawn((
				ChildOf(child),
				TextNode::default(),
				FieldRef::new("root_val").with_document(DocumentPath::Root),
			))
			.id();

		world.update_local();

		// Should resolve to root, not immediate parent
		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(root);

		// Text should be updated
		let content = world.entity(text).get::<TextNode>().unwrap();
		content.0.xpect_eq("from_root");
	}

	#[test]
	fn resolves_card_document_path() {
		let mut world = DocumentPlugin::world();

		// Root without Card marker
		let root = world.spawn(Document::default()).id();
		// Card in the middle
		let card = world
			.spawn((
				ChildOf(root),
				Card,
				Document::new(val!({ "card_val": "from_card" })),
			))
			.id();
		// Nested child
		let child = world.spawn(ChildOf(card)).id();
		let text = world
			.spawn((
				ChildOf(child),
				TextNode::default(),
				FieldRef::new("card_val"), // Default is DocumentPath::Card
			))
			.id();

		world.update_local();

		// Should resolve to card, not root
		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(card);

		let content = world.entity(text).get::<TextNode>().unwrap();
		content.0.xpect_eq("from_card");
	}

	#[test]
	fn resolves_entity_document_path() {
		let mut world = DocumentPlugin::world();

		// Explicit entity reference
		let target = world
			.spawn(Document::new(val!({ "explicit": "target_doc" })))
			.id();

		// Unrelated entity with text
		let text = world
			.spawn((
				TextNode::default(),
				FieldRef::new("explicit")
					.with_document(DocumentPath::Entity(target)),
			))
			.id();

		world.update_local();

		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(target);

		let content = world.entity(text).get::<TextNode>().unwrap();
		content.0.xpect_eq("target_doc");
	}

	#[test]
	fn handles_null_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			Card,
			Document::new(val!({ "nullable": null })),
			children![(TextNode::new("initial"), FieldRef::new("nullable"))],
		));

		world.update_local();

		let text = world.query_once::<&TextNode>()[0].clone();
		text.0.xpect_eq("null");
	}

	#[test]
	fn handles_array_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			Card,
			Document::new(val!({ "items": [1i64, 2i64, 3i64] })),
			children![(TextNode::default(), FieldRef::new("items"))],
		));

		world.update_local();

		let text = world.query_once::<&TextNode>()[0].clone();
		text.0.xpect_eq("[1, 2, 3]");
	}

	#[test]
	fn handles_bool_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((Card, Document::new(val!({ "flag": true })), children![
			(TextNode::default(), FieldRef::new("flag"))
		]));

		world.update_local();

		let text = world.query_once::<&TextNode>()[0].clone();
		text.0.xpect_eq("true");
	}
}
