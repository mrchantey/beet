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
//! `Changed<Document>` is whole-document granularity, but the read path needs
//! per-field granularity: a write to one field dirties the document every other
//! field is bound to, and clobbering all of them would silently discard an edit
//! a sibling field was holding. [`SyncedValue`] records what the document last
//! held for each field, so the read path can tell a change to *this* field from
//! a change to its neighbour and only wins the former.
//!
//! It also breaks the echo. The read path's own write marks the local [`Value`]
//! changed, which the write-back would otherwise propagate straight back; once
//! the document has moved on that echo is a *stale* value overwriting a newer
//! one, and two fields bound to one list will undo each other's appends. A local
//! equal to its [`SyncedValue`] is exactly what the document already knows, so
//! the write-back has nothing to say about it.
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
//! use beet_core::prelude::*;
//!
//! let mut world = DocumentPlugin::world();
//!
//! // Create a document with a value child
//! world.spawn((
//!     Document::new(value!({ "score": 100i64 })),
//!     children![(Value::default(), FieldRef::new("score"))],
//! ));
//!
//! // After update, Value contains Str("100")
//! world.update_local();
//! ```

use crate::prelude::*;

/// Tracks every [`FieldRef`] associated with this [`Document`] entity.
///
/// This component is automatically managed through Bevy's relationship system.
/// The entity may or may not have been initialized with a [`Document`] -
/// the relationship is established based on [`DocumentPath`] resolution.
#[derive(Component)]
#[relationship_target(relationship = FieldOf)]
pub struct Fields(Vec<Entity>);

/// What the document last held for this field, in either sync direction.
///
/// Runtime bookkeeping, deliberately not [`Reflect`](bevy::reflect::Reflect): it
/// is derived from the document and must not ride a scene round trip. Public
/// only because it appears in [`sync_document_to_local`]'s signature, which
/// external systems order against; nothing outside this module inserts it.
///
/// Without it the read path cannot distinguish "the document changed *this*
/// field" from "the document changed *some* field", and a self-bound action's
/// write is discarded whenever any neighbour dirtied the document in the same
/// window.
#[derive(Component)]
pub struct SyncedValue(Value);

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

impl FieldOf {
	/// The resolved `document` when a field on `subject` may link to it now,
	/// `None` when linking must wait.
	///
	/// A self-link only forms when the entity actually owns a [`Document`] (ie a
	/// co-located `(Document, FieldRef)`, as [`BlobStoreList`] does). A
	/// self-resolving ref with no document yet (eg [`DocumentPath::This`] with an
	/// [`OnMissing::Default`]) defers creation to write-back and must not link
	/// prematurely, else the read path would clobber its value.
	///
	/// Shared by the insert-time [`link_field_to_document`] and the reactive
	/// [`update_field_bindings`], so both establish the same link.
	pub(super) fn linkable(
		subject: Entity,
		document: Entity,
		documents: &Query<(), With<Document>>,
	) -> Option<Entity> {
		(document != subject || documents.contains(document))
			.then_some(document)
	}
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
	doc_query: DocumentQuery,
) -> Result {
	let field = fields.get(ev.entity)?;
	let document = doc_query.entity(ev.entity, &field.document);
	let Some(document) = FieldOf::linkable(ev.entity, document, &docs) else {
		return Ok(());
	};
	commands.entity(ev.entity).insert(FieldOf { document });
	Ok(())
}

/// Observer that removes the [`FieldOf`] relationship and the derived
/// [`ResolvedFieldPath`] and [`SyncedValue`] when a [`FieldRef`] is removed.
///
/// All three are derived from the binding, so none outlives it. [`SyncedValue`]
/// in particular records what the document held for *that* field, and a record
/// of a field nothing points at any more is stale by construction.
pub(super) fn unlink_field_from_document(
	ev: On<Remove, FieldRef>,
	mut commands: Commands,
) -> Result {
	commands
		.entity(ev.entity)
		.try_remove::<(FieldOf, ResolvedFieldPath, SyncedValue)>();
	Ok(())
}

/// Read path: when a [`Document`] changes, update the [`Value`] of every
/// [`FieldRef`] bound to it, reading the scope-resolved [`ResolvedFieldPath`].
/// The symmetric counterpart of [`sync_local_to_document`].
///
/// Runs in `PreUpdate` to ensure values are synchronized before user systems run.
///
/// Public so external systems (eg beet_ui's `refresh_blob_store_list`) can order
/// against the document read path.
pub fn sync_document_to_local(
	mut commands: Commands,
	query: Populated<(&Document, &Fields), Changed<Document>>,
	mut text_fields: Query<(
		&ResolvedFieldPath,
		&mut Value,
		Option<&mut SyncedValue>,
	)>,
) -> Result {
	for (doc, doc_fields) in query {
		for field in doc_fields.iter() {
			let Ok((resolved, mut text, synced)) = text_fields.get_mut(field)
			else {
				continue;
			};
			// skip if field not yet present (document may be uninitialized)
			let Ok(field_val) = doc.get_field_ref(&resolved.field_path) else {
				continue;
			};
			match synced {
				// the document holds what it held at the last sync, so this
				// `Changed<Document>` belongs to another field. Leave this one
				// alone: it may be carrying an edit write-back has yet to deliver.
				Some(synced) if synced.0 == *field_val => continue,
				Some(mut synced) => synced.0 = field_val.clone(),
				// no record yet, ie a freshly seeded field: the document wins, which
				// is what the read-before-write chaining has always given it.
				None => {
					commands
						.entity(field)
						.insert(SyncedValue(field_val.clone()));
				}
			}
			if *text != *field_val {
				// only clone if we have to
				*text = field_val.clone();
			}
		}
	}
	Ok(())
}

/// Second document → local sync, gated on a rebound field instead of a changed
/// document.
///
/// Rebinding recomputes a field's derived halves — its [`ResolvedFieldPath`]
/// (a scope change) or its [`FieldOf`] (a [`DocRef`] change or a reparent) —
/// **without** dirtying any document, so [`sync_document_to_local`]'s `Fields`
/// fan-out never fires for it. This re-syncs those fields, reading each field's
/// document via [`FieldOf`].
pub(super) fn sync_rebound_fields(
	changed: Populated<
		(&FieldOf, &ResolvedFieldPath, &mut Value),
		Or<(Changed<ResolvedFieldPath>, Changed<FieldOf>)>,
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

/// Schema read path: reconcile each field's local [`ValueSchema`] with its
/// document's [`DocumentSchema`], the schema-side analog of
/// [`sync_document_to_local`].
///
/// One-directional and lazy: schemas are effectively static after construction,
/// so this resolves a field's schema only on first link (`Added<FieldOf>`) or
/// when the document schema changes, never writing back.
///
/// - a field with no local schema is seeded from the document.
/// - a field with a local schema is asserted to match, erroring on mismatch so a
///   [`TypedFieldRef`] pointed at a differently-typed field is caught rather than
///   silently diverging.
///
/// A document with no [`DocumentSchema`], or a field whose path the schema does
/// not describe, leaves the field-local schema authoritative, mirroring how a
/// document with no value defers to the seeded [`Value`].
/// Run condition gating [`sync_schema`] to frames with a freshly-linked field or
/// a changed document schema, so it does not iterate every frame.
pub(super) fn schema_needs_sync(
	new_links: Query<(), Added<FieldOf>>,
	changed_schemas: Query<(), Changed<DocumentSchema>>,
) -> bool {
	!new_links.is_empty() || !changed_schemas.is_empty()
}

pub(super) fn sync_schema(
	mut commands: Commands,
	fields: Query<(Entity, &FieldOf, &ResolvedFieldPath, Option<&ValueSchema>)>,
	new_links: Query<(), Added<FieldOf>>,
	changed_schemas: Query<(), Changed<DocumentSchema>>,
	schemas: Query<&DocumentSchema>,
) -> Result {
	for (entity, field_of, resolved, local) in fields.iter() {
		// lazy: skip unless the field just linked or its document schema changed
		if !new_links.contains(entity)
			&& !changed_schemas.contains(field_of.document)
		{
			continue;
		}
		// only an inlined schema resolves without a type registry
		let Ok(DocumentSchema(FieldSchema::Inline(schema))) =
			schemas.get(field_of.document)
		else {
			continue;
		};
		// a path the schema does not describe leaves the local schema authoritative
		let Ok(field_schema) = schema.get_field_schema(&resolved.field_path)
		else {
			continue;
		};
		match local {
			Some(local) => {
				local.assert_matches(field_schema, &resolved.field_path)?
			}
			None => {
				commands.entity(entity).insert(field_schema.clone());
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
	mut commands: Commands,
	changed: Populated<
		(
			Entity,
			&FieldRef,
			&ResolvedFieldPath,
			Ref<Value>,
			Option<&SyncedValue>,
		),
		Changed<Value>,
	>,
	mut docs: DocumentQuery,
) -> Result {
	for (entity, field, resolved, value, synced) in changed.iter() {
		// a freshly added Null carries no signal: it must neither clobber a
		// field another binding wrote this pass, nor race a sibling's deferred
		// document creation (the write-back is iteration-order independent).
		if value.is_added() && value.is_null() {
			continue;
		}
		// the local holds exactly what the last sync put there, so this `Changed`
		// is the read path's own write echoing back. Propagating it would push a
		// stale value over whatever the document has since gained.
		if let Some(synced) = synced
			&& synced.0 == *value
		{
			continue;
		}
		// equality guard + policy, computed while the read borrow is live;
		// the guard reads the scope-resolved path, the write scopes internally
		let should_write = match docs.get(entity, &field.document) {
			Ok(doc) => match doc.get_field_ref(&resolved.field_path) {
				// field exists: write only when the value differs
				Ok(field_val) => *field_val != *value,
				// field missing: create it unless the ref opts out
				Err(_) => !matches!(field.on_missing, OnMissing::Error),
			},
			// no document: create one only when the ref initializes on missing
			Err(_) => matches!(field.on_missing, OnMissing::Default(_)),
		};
		if should_write {
			let new = (*value).clone();
			docs.with_field(entity, field, move |slot| *slot = new)?;
			// record what the document now holds, so the next read path can tell
			// this write apart from a neighbour's.
			commands
				.entity(entity)
				.insert(SyncedValue((*value).clone()));
		}
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	fn link_creates_relationship() {
		let mut world = DocumentPlugin::world();

		let card = world.spawn(Document::new(value!({ "x": "value" }))).id();
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

	#[crate::test]
	fn unlink_removes_relationship() {
		let mut world = DocumentPlugin::world();

		let card = world.spawn(Document::new(value!({ "x": "value" }))).id();
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

	#[crate::test]
	fn resolves_root_document_path() {
		let mut world = DocumentPlugin::world();

		let root = world
			.spawn(Document::new(value!({ "root_val": "from_root" })))
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

	#[crate::test]
	fn resolves_card_document_path() {
		let mut world = DocumentPlugin::world();

		// Root document
		let root = world.spawn(Document::default()).id();
		// Nested document in the middle
		let card = world
			.spawn((
				ChildOf(root),
				Document::new(value!({ "card_val": "from_card" })),
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

	#[crate::test]
	fn ancestor_skips_props_document() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(value!({ "name": "user_doc" })))
			.id();
		// a props store between the user doc and the field
		let store = world
			.spawn((
				ChildOf(doc),
				Document::new(value!({ "name": "props_doc" })),
				PropsDocument,
			))
			.id();
		let text = world
			.spawn((ChildOf(store), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		// Ancestor resolution skipped the props store
		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(doc);
		world
			.entity(text)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("user_doc".into()));
	}

	#[crate::test]
	fn resolves_props_document_path() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(value!({ "name": "user_doc" })))
			.id();
		let store = world
			.spawn((
				ChildOf(doc),
				Document::new(value!({ "name": "props_doc" })),
				PropsDocument,
			))
			.id();
		let text = world
			.spawn((
				ChildOf(store),
				Value::default(),
				FieldRef::new("name").with_document(DocumentPath::Props),
			))
			.id();
		world.update_local();

		// Props resolution targeted the store, not the user doc
		let field_of = world.entity(text).get::<FieldOf>().unwrap();
		field_of.document.xpect_eq(store);
		world
			.entity(text)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("props_doc".into()));
	}

	#[crate::test]
	fn nested_props_documents_do_not_leak() {
		let mut world = DocumentPlugin::world();
		// outer store -> inner store: each Props ref resolves its nearest store
		let outer = world
			.spawn((Document::new(value!({ "name": "outer" })), PropsDocument))
			.id();
		let inner = world
			.spawn((
				ChildOf(outer),
				Document::new(value!({ "name": "inner" })),
				PropsDocument,
			))
			.id();
		let inner_field = world
			.spawn((
				ChildOf(inner),
				Value::default(),
				FieldRef::new("name").with_document(DocumentPath::Props),
			))
			.id();
		let outer_field = world
			.spawn((
				ChildOf(outer),
				Value::default(),
				FieldRef::new("name").with_document(DocumentPath::Props),
			))
			.id();
		world.update_local();

		world
			.entity(inner_field)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("inner".into()));
		world
			.entity(outer_field)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Str("outer".into()));
	}

	#[crate::test]
	fn resolves_entity_document_path() {
		let mut world = DocumentPlugin::world();

		// Explicit entity reference
		let target = world
			.spawn(Document::new(value!({ "explicit": "target_doc" })))
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

	#[crate::test]
	fn handles_null_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((Document::new(value!({ "nullable": null })), children![
			(Value::Str("initial".into()), FieldRef::new("nullable"))
		]));

		world.update_local();

		let synced: Vec<_> = world
			.query_once::<(&Value, &FieldRef)>()
			.iter()
			.map(|(val, _)| (*val).clone())
			.collect();
		synced[0].xpect_eq(Value::Null);
	}

	#[crate::test]
	fn handles_array_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((
			Document::new(value!({ "items": [1i64, 2i64, 3i64] })),
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

	#[crate::test]
	fn handles_bool_field_value() {
		let mut world = DocumentPlugin::world();

		world.spawn((Document::new(value!({ "flag": true })), children![(
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
	fn read_field(
		world: &mut World,
		entity: Entity,
		field: &FieldRef,
	) -> Value {
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

	#[crate::test]
	fn value_change_writes_document() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "name": "old" }))).id();
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

	#[crate::test]
	fn converges_in_one_pass_no_loop() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "name": "old" }))).id();
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

	#[crate::test]
	fn document_wins_same_pass_conflict() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "name": "start" }))).id();
		let child = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		// mutate both sides to different values in the same pass
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			value!({ "name": "from_doc" });
		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("from_value".into());
		world.update_local();

		// read-first ordering: the document write wins
		read_field(&mut world, child, &FieldRef::new("name"))
			.xpect_eq(Value::Str("from_doc".into()));
		read_value(&mut world, child).xpect_eq(Value::Str("from_doc".into()));
	}

	/// A write to one field dirties the document every other field is bound to.
	/// The neighbour must not lose the edit it is holding, which is what the
	/// whole-document `Changed` filter used to cost it.
	#[crate::test]
	fn a_neighbours_write_does_not_discard_a_local_edit() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(value!({ "a": "a0", "b": "b0" })))
			.id();
		let field_a = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("a")))
			.id();
		let field_b = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("b")))
			.id();
		world.update_local();

		// a lands first, leaving the document dirty
		*world.entity_mut(field_a).get_mut::<Value>().unwrap() =
			Value::Str("a1".into());
		world.update_local();
		// b is edited while that dirt is still fresh, which is the whole trap
		*world.entity_mut(field_b).get_mut::<Value>().unwrap() =
			Value::Str("b1".into());
		world.update_local();
		world.update_local();

		read_field(&mut world, field_a, &FieldRef::new("a"))
			.xpect_eq(Value::Str("a1".into()));
		read_field(&mut world, field_b, &FieldRef::new("b"))
			.xpect_eq(Value::Str("b1".into()));
	}

	/// Two appends to one list, one pass apart, both land. A guestbook signed
	/// twice in quick succession used to keep only the first entry.
	#[crate::test]
	fn successive_writes_all_land() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "list": [] }))).id();
		let field = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("list")))
			.id();
		world.update_local();

		for entry in ["one", "two", "three"] {
			world
				.entity_mut(field)
				.get_mut::<Value>()
				.unwrap()
				.as_list_mut_or_init()
				.unwrap()
				.push(Value::str(entry));
			world.update_local();
		}

		read_field(&mut world, field, &FieldRef::new("list"))
			.xpect_eq(value!(["one", "two", "three"]));
	}

	/// Two bindings on one field, one writing and one only reading, as a route
	/// that appends beside a route that lists. The reader is updated by the read
	/// path, which marks its local changed; if the write-back then echoes that
	/// value it lands *after* the writer's next append and silently undoes it.
	#[crate::test]
	fn a_reader_does_not_echo_a_stale_value_over_a_writer() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "list": [] }))).id();
		let writer = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("list")))
			.id();
		let reader = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("list")))
			.id();
		world.update_local();

		for entry in ["one", "two", "three"] {
			world
				.entity_mut(writer)
				.get_mut::<Value>()
				.unwrap()
				.as_list_mut_or_init()
				.unwrap()
				.push(Value::str(entry));
			world.update_local();
		}
		world.update_local();

		let expected = value!(["one", "two", "three"]);
		read_field(&mut world, writer, &FieldRef::new("list"))
			.xpect_eq(expected.clone());
		read_value(&mut world, reader).xpect_eq(expected);
	}

	/// Re-pointing an entity at a different field loads the new field, rather
	/// than leaving it showing the old one's value.
	#[crate::test]
	fn repointing_a_field_loads_the_new_one() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(value!({ "a": "a0", "b": "b0" })))
			.id();
		let field = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("a")))
			.id();
		world.update_local();
		read_value(&mut world, field).xpect_eq(Value::Str("a0".into()));

		world.entity_mut(field).remove::<FieldRef>();
		world.entity_mut(field).insert(FieldRef::new("b"));
		world.update_local();
		world.update_local();

		read_value(&mut world, field).xpect_eq(Value::Str("b0".into()));
	}

	#[crate::test]
	fn value_seeds_missing_field() {
		let mut world = DocumentPlugin::world();
		// document present but missing the "name" field
		let doc = world.spawn(Document::new(value!({ "other": 1i64 }))).id();
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

	#[crate::test]
	fn emit_error_missing_field_skipped() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "other": 1i64 }))).id();
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
		document.xpect_eq(value!({ "other": 1i64 }));
	}

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
	fn bidirectional_round_trip() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "name": "start" }))).id();
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
			value!({ "name": "reloaded" });
		world.update_local();
		read_value(&mut world, child).xpect_eq(Value::Str("reloaded".into()));
	}

	/// Clearing a bound field reaches the document: a widget that empties its
	/// value writes `null`, it does not silently keep the old one.
	///
	/// The write-back's one exception is the *freshly added* null every field is
	/// seeded with, which carries no signal and must not clobber the document
	/// before the read path has filled it. A later null is a real edit.
	#[crate::test]
	fn clearing_a_field_reaches_the_document() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "name": "old" }))).id();
		let child = world
			.spawn((ChildOf(doc), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();
		// the seeded null was inert: the read path won
		read_field(&mut world, child, &FieldRef::new("name"))
			.xpect_eq(Value::Str("old".into()));

		// clearing the local value reaches the document as a real null
		*world.entity_mut(child).get_mut::<Value>().unwrap() = Value::Null;
		world.update_local();
		read_field(&mut world, child, &FieldRef::new("name"))
			.xpect_eq(Value::Null);

		// and it stays cleared: nothing restores the old value
		world.update_local();
		read_value(&mut world, child).xpect_eq(Value::Null);
	}

	/// Reparenting a bound field under a different document rebinds it. The link
	/// is derived from ancestry, so a stale one is a widget quietly writing its
	/// edits into the document it used to be under.
	#[crate::test]
	fn reparenting_rebinds_the_document() {
		let mut world = DocumentPlugin::world();
		let first =
			world.spawn(Document::new(value!({ "name": "first" }))).id();
		let second = world
			.spawn(Document::new(value!({ "name": "second" })))
			.id();
		let child = world
			.spawn((ChildOf(first), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();
		read_value(&mut world, child).xpect_eq(Value::Str("first".into()));

		world.entity_mut(child).insert(ChildOf(second));
		world.update_local();

		world
			.entity(child)
			.get::<FieldOf>()
			.unwrap()
			.document
			.xpect_eq(second);
		read_value(&mut world, child).xpect_eq(Value::Str("second".into()));

		// an edit lands in the new document, leaving the old one untouched
		*world.entity_mut(child).get_mut::<Value>().unwrap() =
			Value::Str("edited".into());
		world.update_local();
		world
			.entity(second)
			.get::<Document>()
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!({ "name": "edited" }));
		world
			.entity(first)
			.get::<Document>()
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!({ "name": "first" }));
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn schema_seeds_untyped_field() {
		#[derive(Reflect)]
		#[allow(dead_code)]
		struct CountDoc {
			count: i64,
		}

		let mut world = DocumentPlugin::world();
		// an untyped field beneath a schema-bearing document
		world.spawn((
			Document::default(),
			DocumentSchema::of::<CountDoc>(),
			children![(Value::default(), FieldRef::new("count"))],
		));
		world.update_local();

		// sync_schema seeded the field-local ValueSchema from the document schema
		world
			.query_once::<&ValueSchema>()
			.iter()
			.map(|schema| (*schema).clone())
			.collect::<Vec<_>>()
			.xpect_eq(vec![ValueSchema::of::<i64>()]);
	}
}
