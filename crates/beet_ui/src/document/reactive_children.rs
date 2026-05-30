use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;

/// Reactive structure: spawns one child per item of a list-typed document field,
/// re-spawning them whenever the document changes.
///
/// The companion to [`FieldRef`], which reactively syncs a single [`Value`].
/// Where a [`FieldRef`] tracks one field's value, a [`ReactiveChildren`] tracks
/// a *list* field and materializes a child entity per item via `build_item`.
///
/// Like [`FieldRef`] it links to its document through the [`FieldOf`]
/// relationship, so `Changed<Document>` drives the rebuild. Unlike [`FieldRef`]
/// it does not insert a [`FieldRef`] component, avoiding the `Value` auto-init.
#[derive(Component, Get)]
pub struct ReactiveChildren {
	/// The list-typed field this tracks, ie `DocState::<Vec<_>>::field()`.
	source: FieldRef,
	/// Builds the spawn effect for an item, given its index and [`Value`].
	build_item: Arc<dyn Fn(usize, &Value) -> OnSpawn + Send + Sync>,
	/// The child entities currently owned, despawned and rebuilt on each change.
	owned: Vec<Entity>,
}

impl ReactiveChildren {
	/// Track `source` (a list field), spawning a child per item via `build_item`.
	pub fn new(
		source: FieldRef,
		build_item: impl 'static + Send + Sync + Fn(usize, &Value) -> OnSpawn,
	) -> Self {
		Self {
			source,
			build_item: Arc::new(build_item),
			owned: Vec::new(),
		}
	}
}
/// Observer that links a [`ReactiveChildren`] to its document via [`FieldOf`],
/// so document changes propagate without inserting a [`FieldRef`].
pub(super) fn link_reactive_children_to_document(
	ev: On<Add, ReactiveChildren>,
	mut commands: Commands,
	query: Query<&ReactiveChildren>,
	mut docs: DocumentQuery,
) -> Result {
	let reactive = query.get(ev.entity)?;
	let document = docs.entity(ev.entity, reactive.source().document());
	commands.entity(ev.entity).insert(FieldOf { document });
	Ok(())
}


/// System that rebuilds [`ReactiveChildren`] when their [`Document`] changes.
///
/// Mirrors [`update_text_fields`](super::update_text_fields) to avoid query
/// conflicts: it despawns the previously owned children (cascading), then spawns
/// a fresh child per list item.
pub(super) fn update_reactive_children(
	mut commands: Commands,
	changed: Populated<(&Document, &Fields), Changed<Document>>,
	mut reactive: Query<(Entity, &mut ReactiveChildren)>,
) -> Result {
	for (doc, fields) in changed.iter() {
		for field in fields.iter() {
			let Ok((this, mut reactive)) = reactive.get_mut(field) else {
				continue;
			};
			// despawn the previous generation, children cascade
			for owned in core::mem::take(&mut reactive.owned) {
				commands.entity(owned).despawn();
			}
			// spawn a fresh child per item, recording it for the next rebuild
			if let Ok(Value::List(items)) =
				doc.get_field_ref(&reactive.source.field_path)
			{
				for (index, item) in items.iter().enumerate() {
					let child = commands
						.spawn((
							ChildOf(this),
							(reactive.build_item)(index, item),
						))
						.id();
					reactive.owned.push(child);
				}
			}
		}
	}
	Ok(())
}

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	/// Count the children of `entity`.
	fn child_count(world: &mut World, entity: Entity) -> usize {
		world
			.entity(entity)
			.get::<Children>()
			.map(|children| children.iter().count())
			.unwrap_or(0)
	}

	#[beet_core::test]
	fn grows_and_shrinks() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({ "items": ["a", "b"] })))
			.id();
		let list = world
			.spawn((
				ChildOf(doc),
				ReactiveChildren::new(FieldRef::new("items"), |_, value| {
					OnSpawn::insert(value.clone())
				}),
			))
			.id();
		world.update_local();
		child_count(&mut world, list).xpect_eq(2);

		// grow
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "items": ["a", "b", "c"] });
		world.update_local();
		child_count(&mut world, list).xpect_eq(3);

		// shrink
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "items": ["a"] });
		world.update_local();
		child_count(&mut world, list).xpect_eq(1);
	}

	#[beet_core::test]
	fn leaves_static_siblings_untouched() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({ "items": ["a", "b"] })))
			.id();
		let list = world.spawn((ChildOf(doc),)).id();
		// a static sibling that ReactiveChildren must never despawn
		let static_child = world
			.spawn((ChildOf(list), Value::Str("static".into())))
			.id();
		world.entity_mut(list).insert(ReactiveChildren::new(
			FieldRef::new("items"),
			|_, value| OnSpawn::insert(value.clone()),
		));
		world.update_local();
		// two owned plus the static sibling
		child_count(&mut world, list).xpect_eq(3);

		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "items": [] });
		world.update_local();
		// owned despawned, static sibling survives
		child_count(&mut world, list).xpect_eq(1);
		world.entities().contains(static_child).xpect_true();
	}
}
