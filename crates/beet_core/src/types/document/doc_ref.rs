//! [`DocRef`]: the relation binding a subtree's fields into a foreign document.
use crate::prelude::*;

/// The document every [`FieldRef`] beneath this entity binds into: the source
/// half of the [`DocConsumers`] relationship.
///
/// [`DocumentPath::Ancestor`], the default, walks up until it finds a document.
/// A `DocRef` on that walk ends it, redirecting the whole subtree at the entity
/// it targets wherever that entity lives in the tree, so a form nested anywhere
/// edits a document it is not under: the schema editor binding the schema
/// document, or a row's detail form binding that row's slice of the data
/// document.
///
/// It is the only markup-authorable way to name a foreign document; the
/// [`DocumentPath::Entity`] it resolves to is resolution's output, never an
/// authored input. Target it by `bx:ref`, exactly like `StoreRef`:
///
/// ```html
/// <Fragment bx:ref="schema" {schema.bundle()}/>
/// <SchemaEditor {DocRef($schema)}/>
/// ```
///
/// A `DocRef` co-located with a [`Document`] wins over it: the redirect is
/// deliberate where the storage is incidental.
///
/// ## Loud, never a fallback
///
/// Ending the walk is the point. Nothing falls back to the ancestor document, so
/// a target that answers no [`Document`] binds a subtree that syncs nothing
/// rather than one that writes its edits into the host document. The moment such
/// a field is written the write fails naming the relation, instead of creating a
/// stray document at the target ([`DocumentQuery::with_field`]).
///
/// Reads stay lenient because a document legitimately arrives late (a store read
/// resolving frames after the tree is built); a target that never answers one is
/// caught by the first write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = DocConsumers, allow_self_referential)]
pub struct DocRef(#[entities] pub Entity);

impl DocRef {
	/// The document entity this subtree is bound to.
	pub fn document(&self) -> Entity { self.0 }
}

/// Every subtree bound to this document: the target half of the [`DocRef`]
/// relationship, on the document entity.
///
/// Doubles as the mark that makes a missing document loud. An entity nothing
/// declared may have a document created on it by an initializing [`FieldRef`];
/// an entity a `DocRef` names was declared to already be one, so the same write
/// is an error there.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = DocRef)]
pub struct DocConsumers(Vec<Entity>);

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	/// The local [`Value`] of `entity`.
	fn read_value(world: &mut World, entity: Entity) -> Value {
		world.entity(entity).get::<Value>().unwrap().clone()
	}

	/// A world with a `host` document and a detached `foreign` document, the
	/// two-document shape every test here binds across.
	fn worlds() -> (World, Entity, Entity) {
		let mut world = DocumentPlugin::world();
		let host = world.spawn(Document::new(value!({ "name": "host" }))).id();
		let foreign = world
			.spawn(Document::new(value!({ "name": "foreign" })))
			.id();
		(world, host, foreign)
	}

	#[crate::test]
	fn binds_the_targeted_document() {
		let (mut world, host, foreign) = worlds();
		// the ref sits under the host document, but reads the foreign one
		let subtree = world.spawn((ChildOf(host), DocRef(foreign))).id();
		let field = world
			.spawn((ChildOf(subtree), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		world
			.entity(field)
			.get::<FieldOf>()
			.unwrap()
			.document
			.xpect_eq(foreign);
		read_value(&mut world, field).xpect_eq(Value::Str("foreign".into()));
	}

	/// The redirect is a write path too: an edit lands in the targeted document
	/// and never touches the host the widget is nested under.
	#[crate::test]
	fn writes_reach_the_targeted_document() {
		let (mut world, host, foreign) = worlds();
		let subtree = world.spawn((ChildOf(host), DocRef(foreign))).id();
		let field = world
			.spawn((ChildOf(subtree), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		*world.entity_mut(field).get_mut::<Value>().unwrap() =
			Value::Str("edited".into());
		world.update_local();

		world
			.entity(foreign)
			.get::<Document>()
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!({ "name": "edited" }));
		// the host document the subtree is nested under is untouched
		world
			.entity(host)
			.get::<Document>()
			.unwrap()
			.0
			.clone()
			.xpect_eq(value!({ "name": "host" }));
	}

	/// The nearest declaration wins, so a `DocRef` inside a `DocRef` subtree
	/// rebinds its own descendants without leaking outward.
	#[crate::test]
	fn nearest_ref_wins() {
		let (mut world, host, foreign) = worlds();
		let inner_doc =
			world.spawn(Document::new(value!({ "name": "inner" }))).id();
		let outer = world.spawn((ChildOf(host), DocRef(foreign))).id();
		let inner = world.spawn((ChildOf(outer), DocRef(inner_doc))).id();
		let outer_field = world
			.spawn((ChildOf(outer), Value::default(), FieldRef::new("name")))
			.id();
		let inner_field = world
			.spawn((ChildOf(inner), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();

		read_value(&mut world, outer_field)
			.xpect_eq(Value::Str("foreign".into()));
		read_value(&mut world, inner_field)
			.xpect_eq(Value::Str("inner".into()));
	}

	/// A `DocRef` added above existing refs rebinds them: the link is derived,
	/// so a stale one would silently keep writing the host document.
	#[crate::test]
	fn added_ref_rebinds_existing_fields() {
		let (mut world, host, foreign) = worlds();
		let subtree = world.spawn(ChildOf(host)).id();
		let field = world
			.spawn((ChildOf(subtree), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();
		read_value(&mut world, field).xpect_eq(Value::Str("host".into()));

		world.entity_mut(subtree).insert(DocRef(foreign));
		world.update_local();

		world
			.entity(field)
			.get::<FieldOf>()
			.unwrap()
			.document
			.xpect_eq(foreign);
		read_value(&mut world, field).xpect_eq(Value::Str("foreign".into()));
	}

	/// Removing the declaration re-resolves the subtree to the ancestor walk.
	#[crate::test]
	fn removed_ref_rebinds_to_the_ancestor() {
		let (mut world, host, foreign) = worlds();
		let subtree = world.spawn((ChildOf(host), DocRef(foreign))).id();
		let field = world
			.spawn((ChildOf(subtree), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();
		read_value(&mut world, field).xpect_eq(Value::Str("foreign".into()));

		world.entity_mut(subtree).remove::<DocRef>();
		world.update_local();

		world
			.entity(field)
			.get::<FieldOf>()
			.unwrap()
			.document
			.xpect_eq(host);
		read_value(&mut world, field).xpect_eq(Value::Str("host".into()));
	}

	/// A scope above the declaration belongs to the host document's namespace,
	/// so it must not prefix paths into the foreign one; a scope beneath it must.
	#[crate::test]
	fn scopes_stop_at_the_declaration() {
		let mut world = DocumentPlugin::world();
		let host = world
			.spawn(Document::new(value!({ "outer": { "name": "host" } })))
			.id();
		let foreign = world
			.spawn(Document::new(value!({
				"name": "foreign",
				"inner": { "name": "scoped" }
			})))
			.id();
		let outer_scope = world
			.spawn((ChildOf(host), DocumentScope {
				path: FieldPath::new(["outer"]),
				terminate: false,
			}))
			.id();
		let subtree = world.spawn((ChildOf(outer_scope), DocRef(foreign))).id();
		let unscoped = world
			.spawn((ChildOf(subtree), Value::default(), FieldRef::new("name")))
			.id();
		let inner_scope = world
			.spawn((ChildOf(subtree), DocumentScope {
				path: FieldPath::new(["inner"]),
				terminate: false,
			}))
			.id();
		let scoped = world
			.spawn((
				ChildOf(inner_scope),
				Value::default(),
				FieldRef::new("name"),
			))
			.id();
		world.update_local();

		// the host's "outer" prefix stopped at the declaration
		read_value(&mut world, unscoped).xpect_eq(Value::Str("foreign".into()));
		// a scope inside the subtree still applies, in the foreign namespace
		read_value(&mut world, scoped).xpect_eq(Value::Str("scoped".into()));
	}

	/// A target that answers no document syncs nothing rather than falling back,
	/// and the first write names the relation instead of creating a stray
	/// document there.
	#[crate::test]
	#[should_panic = "DocRef"]
	fn a_target_with_no_document_fails_the_write() {
		let mut world = DocumentPlugin::world();
		let host = world.spawn(Document::new(value!({ "name": "host" }))).id();
		// a `bx:ref` naming a node that never carried a document
		let dangling = world.spawn_empty().id();
		let subtree = world.spawn((ChildOf(host), DocRef(dangling))).id();
		let field = world
			.spawn((ChildOf(subtree), Value::default(), FieldRef::new("name")))
			.id();
		world.update_local();
		// no silent fallback: the host document did not answer the read
		read_value(&mut world, field).xpect_eq(Value::Null);

		*world.entity_mut(field).get_mut::<Value>().unwrap() =
			Value::Str("edited".into());
		world.update_local();
	}
}
