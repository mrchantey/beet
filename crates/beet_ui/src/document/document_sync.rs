//! Bidirectional document synchronization for field values.
//!
//! This module keeps [`Value`] components in sync with their associated
//! [`Document`] fields through [`FieldRef`], in both directions:
//!
//! - `sync_document_to_local` (document → [`Value`]): the read path.
//! - `sync_local_to_document` ([`Value`] → document): the symmetric write-back.
//!
//! Both directions gate on inequality, so once the two sides agree neither
//! writes and the loop settles. The read path is chained before the write-back,
//! so on initial insert the document's real value lands in the seeded [`Value`]
//! before write-back runs, and a same-pass conflict resolves document-wins.
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
//! 3. When a [`Document`] changes, the `sync_document_to_local` system iterates
//!    through all related [`FieldRef`] entities and updates their [`Value`].
//!
//! 4. When a synced [`Value`] changes, the `sync_local_to_document` system
//!    writes it back into the resolved document field.
//!
//! # Example
//!
//! ```ignore
//! use beet_ui::prelude::*;
//! use beet_core::prelude::*;
//!
//! let mut world = DocumentPlugin::world();
//!
//! // Create a document with a value child
//! world.spawn((
//!     Document::new(val!({ "score": 100i64 })),
//!     children![(Value::default(), FieldRef::new("score"))],
//! ));
//!
//! // After update, Value contains Str("100")
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
///
/// `allow_self_referential` so a [`FieldRef`] co-located with its [`Document`]
/// (ie `(Document, FieldRef)` on one entity, as [`BlobStoreList`] does) still
/// links and syncs.
#[derive(Component)]
#[relationship(relationship_target = Fields, allow_self_referential)]
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
	docs: Query<(), With<Document>>,
	mut doc_query: DocumentQuery,
) -> Result {
	let field = fields.get(ev.entity)?;
	let document = doc_query.entity(ev.entity, &field.document);
	// a self-link only forms when the entity actually owns a Document (ie a
	// co-located `(Document, FieldRef)`). A self-resolving ref with no Document
	// yet (eg `DocumentPath::This` + Init) defers creation to write-back and
	// must not link prematurely, else the read path would clobber its value.
	if document == ev.entity && !docs.contains(document) {
		return Ok(());
	}
	commands.entity(ev.entity).insert(FieldOf { document });
	Ok(())
}

/// Observer that removes the [`FieldOf`] relationship and the derived
/// [`ResolvedFieldPath`] when a [`FieldRef`] is removed.
pub(super) fn unlink_field_from_document(
	ev: On<Remove, FieldRef>,
	mut commands: Commands,
) -> Result {
	commands
		.entity(ev.entity)
		.try_remove::<(FieldOf, ResolvedFieldPath)>();
	Ok(())
}


/// Read path: when a [`Document`] changes, update the [`Value`] of every
/// [`FieldRef`] bound to it, reading the scope-resolved [`ResolvedFieldPath`].
/// The symmetric counterpart of [`sync_local_to_document`].
///
/// Runs in `PreUpdate` to ensure values are synchronized before user systems run.
pub(super) fn sync_document_to_local(
	query: Populated<(&Document, &Fields), Changed<Document>>,
	mut text_fields: Query<(&ResolvedFieldPath, &mut Value)>,
) -> Result {
	for (doc, doc_fields) in query {
		for field in doc_fields.iter() {
			if let Ok((resolved, mut text)) = text_fields.get_mut(field) {
				// skip if field not yet present (document may be uninitialized)
				if let Ok(field_val) = doc.get_field_ref(&resolved.field_path) {
					if *text != *field_val {
						// only clone if we have to
						*text = field_val.clone();
					}
				}
			}
		}
	}
	Ok(())
}

/// Second document → local sync, gated on `Changed<ResolvedFieldPath>` instead
/// of `Changed<Document>`.
///
/// A scope change recomputes [`ResolvedFieldPath`] **without** dirtying the
/// document, so [`sync_document_to_local`]'s `Fields` fan-out never fires for it.
/// This re-syncs those fields, reading each field's document via [`FieldOf`].
pub(super) fn sync_resolved_path_changes(
	changed: Populated<
		(&FieldOf, &ResolvedFieldPath, &mut Value),
		Changed<ResolvedFieldPath>,
	>,
	docs: Query<&Document>,
) -> Result {
	for (field_of, resolved, mut value) in changed {
		let Ok(doc) = docs.get(field_of.document) else {
			continue;
		};
		if let Ok(field_val) = doc.get_field_ref(&resolved.field_path) {
			if *value != *field_val {
				*value = field_val.clone();
			}
		}
	}
	Ok(())
}

/// Write-back: when a field-bound entity's local [`Value`] changes, propagate it
/// into the resolved document field. The symmetric counterpart of
/// [`sync_document_to_local`]; the equality guard on both directions is what
/// breaks the otherwise-infinite sync loop.
pub(super) fn sync_local_to_document(
	changed: Populated<
		(Entity, &FieldRef, &ResolvedFieldPath, &Value),
		Changed<Value>,
	>,
	mut docs: DocumentQuery,
) -> Result {
	for (entity, field, resolved, value) in changed.iter() {
		// equality guard + policy, computed while the read borrow is live;
		// the guard reads the scope-resolved path, the write scopes internally
		let should_write = match docs.get(entity, &field.document) {
			Ok(doc) => match doc.get_field_ref(&resolved.field_path) {
				// field exists: write only when it differs
				Ok(field_val) => *field_val != *value,
				// field missing: create it unless the ref opts out
				Err(_) => !matches!(field.on_missing, OnMissingField::EmitError),
			},
			// no document: create one only when the ref initializes on missing
			Err(_) => matches!(field.on_missing, OnMissingField::Init { .. }),
		};
		if should_write {
			let new = value.clone();
			docs.with_field(entity, field, move |slot| *slot = new)?;
		}
	}
	Ok(())
}


#[cfg(test)]
mod test {
	use super::*;

	#[beet_core::test]
	fn link_creates_relationship() {
		let mut world = DocumentPlugin::world();

		let card = world.spawn(Document::new(val!({ "x": "value" }))).id();
		let text = world
			.spawn((ChildOf(card), Value::default(), FieldRef::new("x")))
			.id();

		world.update_local();

		// FieldOf should point to the card
		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(card);

		// Document entity should have Fields tracking the text entity
		let fields = world.entity(card).get::<Fields>().unwrap();
		fields
			.iter()
			.collect::<Vec<_>>()
			.contains(&text)
			.xpect_true();
	}

	#[beet_core::test]
	fn unlink_removes_relationship() {
		let mut world = DocumentPlugin::world();

		let card = world.spawn(Document::new(val!({ "x": "value" }))).id();
		let text = world
			.spawn((ChildOf(card), Value::default(), FieldRef::new("x")))
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

	#[beet_core::test]
	fn resolves_root_document_path() {
		let mut world = DocumentPlugin::world();

		let root = world
			.spawn(Document::new(val!({ "root_val": "from_root" })))
			.id();
		let child = world.spawn(ChildOf(root)).id();
		let text = world
			.spawn((
				ChildOf(child),
				Value::default(),
				FieldRef::new("root_val").with_document(DocumentPath::Root),
			))
			.id();

		world.update_local();

		// Should resolve to root, not immediate parent
		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(root);

		// Value should be updated
		let content = world.entity(text).get::<Value>().unwrap().clone();
		content.xpect_eq(Value::Str("from_root".into()));
	}

	#[beet_core::test]
	fn resolves_card_document_path() {
		let mut world = DocumentPlugin::world();

		// Root document
		let root = world.spawn(Document::default()).id();
		// Nested document in the middle
		let card = world
			.spawn((
				ChildOf(root),
				Document::new(val!({ "card_val": "from_card" })),
			))
			.id();
		// Nested child
		let child = world.spawn(ChildOf(card)).id();
		let text = world
			.spawn((
				ChildOf(child),
				Value::default(),
				FieldRef::new("card_val"), // Default is DocumentPath::Ancestor
			))
			.id();

		world.update_local();

		// Should resolve to card, not root
		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(card);

		let content = world.entity(text).get::<Value>().unwrap().clone();
		content.xpect_eq(Value::Str("from_card".into()));
	}

	#[beet_core::test]
	fn resolves_entity_document_path() {
		let mut world = DocumentPlugin::world();

		// Explicit entity reference
		let target = world
			.spawn(Document::new(val!({ "explicit": "target_doc" })))
			.id();

		// Unrelated entity with value
		let text = world
			.spawn((
				Value::default(),
				FieldRef::new("explicit")
					.with_document(DocumentPath::Entity(target)),
			))
			.id();

		world.update_local();

		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(target);

		let content = world.entity(text).get::<Value>().unwrap().clone();
		content.xpect_eq(Value::Str("target_doc".into()));
	}

	#[beet_core::test]
	fn handles_null_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((Document::new(val!({ "nullable": null })), children![(
			Value::Str("initial".into()),
			FieldRef::new("nullable")
		)]));

		world.update_local();

		let synced: Vec<_> = world
			.query_once::<(&Value, &FieldRef)>()
			.iter()
			.map(|(val, _)| (*val).clone())
			.collect();
		synced[0].xpect_eq(Value::Null);
	}

	#[beet_core::test]
	fn handles_array_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			Document::new(val!({ "items": [1i64, 2i64, 3i64] })),
			children![(Value::default(), FieldRef::new("items"))],
		));

		world.update_local();

		let synced: Vec<_> = world
			.query_once::<(&Value, &FieldRef)>()
			.iter()
			.map(|(val, _)| (*val).clone())
			.collect();
		synced[0].xpect_eq(Value::new_list([1, 2, 3]));
	}

	#[beet_core::test]
	fn handles_bool_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((Document::new(val!({ "flag": true })), children![(
			Value::default(),
			FieldRef::new("flag")
		)]));

		world.update_local();

		let synced: Vec<_> = world
			.query_once::<(&Value, &FieldRef)>()
			.iter()
			.map(|(val, _)| (*val).clone())
			.collect();
		synced[0].xpect_eq(Value::Bool(true));
	}

	/// Read the resolved document field of `field` as seen from `entity`.
	fn read_field(world: &mut World, entity: Entity, field: &FieldRef) -> Value {
		world
			.run_system_cached_with(
				|In((entity, field)): In<(Entity, FieldRef)>,
				 mut docs: DocumentQuery| {
					docs.get(entity, &field.document)
						.unwrap()
						.get_field_ref(&field.field_path)
						.unwrap()
						.clone()
				},
				(entity, field.clone()),
			)
			.unwrap()
	}

	/// Read the local [`Value`] of `entity`.
	fn read_value(world: &mut World, entity: Entity) -> Value {
		world.entity(entity).get::<Value>().unwrap().clone()
	}

	#[beet_core::test]
	fn value_change_writes_document() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "name": "old" }))).id();
		let child = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		// read path loaded the document value into the local Value
		read_value(&mut world, child).xpect_eq(Value::Str("old".into()));

		// mutate the local Value, write-back should reach the document
		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("new".into());
		world.update_local();

		read_field(&mut world, child, &FieldRef::new("name"))
			.xpect_eq(Value::Str("new".into()));
	}

	#[beet_core::test]
	fn converges_in_one_pass_no_loop() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "name": "old" }))).id();
		let child = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("new".into());
		world.update_local();

		// after a single pass both sides agree
		let field = FieldRef::new("name");
		read_field(&mut world, child, &field)
			.xpect_eq(Value::Str("new".into()));
		read_value(&mut world, child).xpect_eq(Value::Str("new".into()));

		// further passes must not drift or oscillate
		for _ in 0..3 {
			world.update_local();
			read_field(&mut world, child, &field)
				.xpect_eq(Value::Str("new".into()));
			read_value(&mut world, child).xpect_eq(Value::Str("new".into()));
		}
	}

	#[beet_core::test]
	fn document_wins_same_pass_conflict() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "name": "start" }))).id();
		let child = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		// mutate both sides to different values in the same pass
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "name": "from_doc" });
		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("from_value".into());
		world.update_local();

		// read-first ordering: the document write wins
		read_field(&mut world, child, &FieldRef::new("name"))
			.xpect_eq(Value::Str("from_doc".into()));
		read_value(&mut world, child).xpect_eq(Value::Str("from_doc".into()));
	}

	#[beet_core::test]
	fn value_seeds_missing_field() {
		let mut world = DocumentPlugin::world();
		// document present but missing the "name" field
		let doc = world.spawn(Document::new(val!({ "other": 1i64 }))).id();
		let child = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("name")))
			.id();
		// settle the seed so the document's changed flag ages out before the edit
		world.update_local();
		world.update_local();

		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("seeded".into());
		world.update_local();

		read_field(&mut world, child, &FieldRef::new("name"))
			.xpect_eq(Value::Str("seeded".into()));
	}

	#[beet_core::test]
	fn emit_error_missing_field_skipped() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "other": 1i64 }))).id();
		let child = world
			.spawn((
				ChildOf(doc),
				Value::default(),
				FieldRef::new("name").error_on_missing(),
			))
			.id();
		world.update_local();

		// mutating the local Value must not error or touch the document
		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("ignored".into());
		world.update_local();

		let document = world.entity(doc).get::<Document>().unwrap().0.clone();
		document.xpect_eq(val!({ "other": 1i64 }));
	}

	#[beet_core::test]
	fn no_document_init_creates() {
		let mut world = DocumentPlugin::world();
		// a lone FieldRef child with Init resolving via This, no ancestor Document
		let child = world
			.spawn((
				Value::default(),
				FieldRef::new("name").with_document(DocumentPath::This),
			))
			.id();
		world.update_local();

		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("created".into());
		world.update_local();

		// write-back materialized a Document on the resolved entity
		world.entity(child).contains::<Document>().xpect_true();
		read_field(
			&mut world,
			child,
			&FieldRef::new("name").with_document(DocumentPath::This),
		)
		.xpect_eq(Value::Str("created".into()));
	}

	#[beet_core::test]
	fn no_document_emit_error_skips() {
		let mut world = DocumentPlugin::world();
		let child = world
			.spawn((
				Value::default(),
				FieldRef::new("name")
					.with_document(DocumentPath::This)
					.error_on_missing(),
			))
			.id();
		world.update_local();

		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("ignored".into());
		world.update_local();

		// no Document conjured anywhere
		world.entity(child).contains::<Document>().xpect_false();
		world.query_once::<&Document>().is_empty().xpect_true();
	}

	#[beet_core::test]
	fn bidirectional_round_trip() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(val!({ "name": "start" }))).id();
		let child = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("name")))
			.id();

		// document → Value (read path)
		world.update_local();
		read_value(&mut world, child).xpect_eq(Value::Str("start".into()));

		// Value → document (write-back)
		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("edited".into());
		world.update_local();
		read_field(&mut world, child, &FieldRef::new("name"))
			.xpect_eq(Value::Str("edited".into()));

		// document → Value again, proving the loop is alive in both directions
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "name": "reloaded" });
		world.update_local();
		read_value(&mut world, child).xpect_eq(Value::Str("reloaded".into()));
	}
}
