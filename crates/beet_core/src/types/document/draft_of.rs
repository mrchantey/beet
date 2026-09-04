//! [`DraftOf`]: the relation from a document to the one it was forked from.
use crate::prelude::*;

/// The document this one is a draft of: the source half of the [`Drafts`]
/// relationship.
///
/// A draft is an ordinary [`Document`] that started as a copy of another and is
/// then edited freely, so every widget binding it is an ordinary bound widget
/// and nothing writes into the origin until something commits. That is what
/// makes a transactional edit expressible at all: a schema half-typed into its
/// own document would already have invalidated the data it describes
/// (`SchemaCommit` is the boundary that judgement belongs to).
///
/// **The origin is a relation, not a copy.** Forking and merging is the shape
/// a CRDT (the parked history/collaboration workstream) natively speaks, so the
/// link is what survives a change of merge policy; what a policy needs beyond
/// it — a base snapshot, a change log — is that policy's private data rather
/// than this API. Today's policy is the simplest one: a draft is forked once and
/// is thereafter **sticky**, so an origin that moves on does not drag the
/// draft's edits away, and [`RevertDraft`] is how an author discards them.
/// Deliberately absent is any treatment of "the origin moved" as fatal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = Drafts)]
// a draft is a document from the moment it is declared one, so the subtree
// binding it never falls through to the document above while the fork is still
// arriving.
#[require(Document)]
pub struct DraftOf(#[entities] pub Entity);

impl DraftOf {
	/// The document this draft was forked from.
	pub fn origin(&self) -> Entity { self.0 }
}

/// Every draft forked from this document: the target half of the [`DraftOf`]
/// relationship, on the origin.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = DraftOf)]
pub struct Drafts(Vec<Entity>);

/// Marks a draft that has been forked, so the copy happens exactly once however
/// late the origin arrives.
///
/// Runtime bookkeeping of the current merge policy, deliberately not
/// [`Reflect`](bevy::reflect::Reflect): a reloaded scene re-forks from whatever
/// its origin holds then.
#[derive(Component)]
pub(super) struct Forked;

/// Fired on a draft to fork it from its origin again, discarding what was
/// drafted.
///
/// The explicit half of a sticky draft: nothing else ever overwrites an edit in
/// progress, so an author who wants the origin back asks for it.
#[derive(Debug, EntityEvent)]
pub struct RevertDraft {
	/// The draft to re-fork.
	#[event_target]
	pub draft: Entity,
}

/// Copy each unforked draft's origin into it, once its origin has a document.
///
/// Runs at the head of the sync chain, so a fresh fork reaches every widget
/// bound to the draft in the same pass. A draft whose origin has no document yet
/// is left alone rather than seeded empty: a document legitimately arrives late
/// (a store read resolving frames after the tree is built), which is the same
/// lenience [`DocRef`] resolution settled on.
pub(super) fn fork_drafts(
	drafts: Populated<(Entity, &DraftOf), Without<Forked>>,
	documents: Query<&Document>,
	mut commands: Commands,
) {
	for (entity, draft_of) in drafts.iter() {
		let Ok(origin) = documents.get(draft_of.origin()) else {
			continue;
		};
		commands.entity(entity).insert((origin.clone(), Forked));
	}
}

/// Observer: [`RevertDraft`] re-forks a draft from its origin.
pub(super) fn revert_draft(
	ev: On<RevertDraft>,
	drafts: Query<&DraftOf>,
	mut documents: Query<&mut Document>,
) -> Result {
	let origin = drafts.get(ev.draft)?.origin();
	let origin = documents.get(origin)?.clone();
	documents.get_mut(ev.draft)?.set_if_neq(origin);
	OK
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	/// An origin and a draft of it, settled.
	fn worlds() -> (World, Entity, Entity) {
		let mut world = DocumentPlugin::world();
		let origin = world.spawn(Document::new(value!({ "name": "ada" }))).id();
		let draft = world.spawn(DraftOf(origin)).id();
		world.update_local();
		(world, origin, draft)
	}

	fn document(world: &mut World, entity: Entity) -> Value {
		world.entity(entity).get::<Document>().unwrap().0.clone()
	}

	#[crate::test]
	fn a_draft_forks_its_origin() {
		let (mut world, _, draft) = worlds();
		document(&mut world, draft).xpect_eq(value!({ "name": "ada" }));
	}

	/// The fork happens once: an edited draft is not dragged along by its
	/// origin, which is what makes it a place to work rather than a mirror.
	#[crate::test]
	fn a_draft_is_sticky() {
		let (mut world, origin, draft) = worlds();
		world.entity_mut(draft).get_mut::<Document>().unwrap().0 =
			value!({ "name": "drafted" });
		world.entity_mut(origin).get_mut::<Document>().unwrap().0 =
			value!({ "name": "moved on" });
		world.update_local();
		document(&mut world, draft).xpect_eq(value!({ "name": "drafted" }));
	}

	/// Reverting is the explicit way back, and it reads the origin as it stands
	/// *now* rather than as it stood at the fork.
	#[crate::test]
	fn reverting_re_forks_the_origin() {
		let (mut world, origin, draft) = worlds();
		world.entity_mut(draft).get_mut::<Document>().unwrap().0 =
			value!({ "name": "drafted" });
		world.entity_mut(origin).get_mut::<Document>().unwrap().0 =
			value!({ "name": "moved on" });
		world.update_local();

		world.trigger(RevertDraft { draft });
		world.flush();
		document(&mut world, draft).xpect_eq(value!({ "name": "moved on" }));
	}

	/// An origin that has not answered a document yet leaves the draft empty
	/// rather than forking nothing, and forks the moment it arrives.
	#[crate::test]
	fn a_late_origin_still_forks() {
		let mut world = DocumentPlugin::world();
		let origin = world.spawn_empty().id();
		let draft = world.spawn(DraftOf(origin)).id();
		world.update_local();
		document(&mut world, draft).xpect_eq(Value::default());

		world
			.entity_mut(origin)
			.insert(Document::new(value!({ "name": "late" })));
		world.update_local();
		document(&mut world, draft).xpect_eq(value!({ "name": "late" }));
	}
}
