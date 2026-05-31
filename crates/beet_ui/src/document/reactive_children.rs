use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;

/// Reactive structure: spawns one child per item of a list-typed document field,
/// re-spawning them whenever that field's [`Value`] changes.
///
/// The companion to [`FieldRef`], which reactively syncs a single [`Value`].
/// Where a [`FieldRef`] tracks one field's value, a [`ReactiveChildren`] tracks
/// a *list* field and materializes a child entity per item via `build_item`.
///
/// It rides the existing [`FieldRef`] machinery rather than duplicating it: the
/// author spawns it via [`new`](Self::new), which pairs it with a [`FieldRef`].
/// That ref links to the document and inserts the synced [`Value`], so a
/// `Changed<Value>` on this entity drives the rebuild. Each spawned child is
/// tagged [`ReactiveChild`] so a rebuild despawns exactly the previous
/// generation and leaves static siblings alone.
#[derive(Component)]
pub struct ReactiveChildren {
	/// Builds the spawn effect for an item, given its index and [`Value`].
	build_item: Arc<dyn Fn(usize, &Value) -> OnSpawn + Send + Sync>,
}

/// Marker on each child spawned by a [`ReactiveChildren`], so a rebuild can
/// despawn exactly the previous generation and leave static siblings alone.
#[derive(Component)]
pub struct ReactiveChild;

impl ReactiveChildren {
	/// Track `field` (a list field), spawning a child per item via `build_item`.
	///
	/// Returns a bundle pairing the [`FieldRef`] with the [`ReactiveChildren`]:
	/// the ref's `on_add` inserts the synced [`Value`] and `sync_document_to_local`
	/// keeps it current, so the rebuild rides `Changed<Value>`.
	pub fn new(
		field: FieldRef,
		build_item: impl 'static + Send + Sync + Fn(usize, &Value) -> OnSpawn,
	) -> impl Bundle {
		(field, ReactiveChildren {
			build_item: Arc::new(build_item),
		})
	}
}

/// System that rebuilds [`ReactiveChildren`] when their synced [`Value`] changes.
///
/// Chained after [`sync_document_to_local`](super::sync_document_to_local),
/// which writes the [`Value`] and marks it `Changed`, so the rebuild reads the
/// current list the same pass, including the initial generation.
pub(super) fn update_reactive_children(
	mut commands: Commands,
	changed: Populated<
		(
			Entity,
			&Value,
			&ResolvedFieldPath,
			&ReactiveChildren,
			Option<&Children>,
		),
		Changed<Value>,
	>,
	reactive_children: Query<(), With<ReactiveChild>>,
) -> Result {
	for (entity, value, resolved, reactive, children) in changed.iter() {
		// despawn the previous generation: this entity's ReactiveChild children
		if let Some(children) = children {
			children
				.iter()
				.filter(|child| reactive_children.contains(*child))
				.for_each(|child| commands.entity(child).despawn());
		}
		// spawn the new generation, each tagged ReactiveChild, via OnSpawn
		if let Ok(items) = value.as_list() {
			for (index, item) in items.iter().enumerate() {
				commands.spawn((
					ChildOf(entity),
					ReactiveChild,
					// the child's fully-resolved absolute item path, terminating
					// so an inner FieldRef does not double-count outer scopes
					DocumentScope {
						path: resolved.field_path.with_pushed(index),
						terminate: true,
					},
					(reactive.build_item)(index, item),
				));
			}
		}
	}
	Ok(())
}

#[cfg(all(test, feature = "json"))]
mod test {
	use super::*;

	/// Count the `ReactiveChild` children of `entity`.
	fn child_count(world: &mut World, entity: Entity) -> usize {
		let children: Vec<Entity> = world
			.entity(entity)
			.get::<Children>()
			.map(|children| children.iter().collect())
			.unwrap_or_default();
		children
			.iter()
			.filter(|child| world.entity(**child).contains::<ReactiveChild>())
			.count()
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
		let list = world
			.spawn((
				ChildOf(doc),
				ReactiveChildren::new(FieldRef::new("items"), |_, value| {
					OnSpawn::insert(value.clone())
				}),
			))
			.id();
		// a static, non-ReactiveChild sibling that must never be despawned
		let static_child = world
			.spawn((ChildOf(list), Value::Str("static".into())))
			.id();
		world.update_local();
		// two reactive children, the static sibling is not counted
		child_count(&mut world, list).xpect_eq(2);

		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "items": [] });
		world.update_local();
		// reactive children despawned, static sibling survives
		child_count(&mut world, list).xpect_eq(0);
		world.entities().contains(static_child).xpect_true();
	}

	#[beet_core::test]
	fn rebuilds_only_on_own_value_change() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({ "items": ["a"] })))
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
		child_count(&mut world, list).xpect_eq(1);
		let generation = world.entity(list).get::<Children>().unwrap()[0];

		// an unrelated document changing must not rebuild this list
		world.spawn(Document::new(val!({ "other": 1i64 })));
		world.update_local();
		// the same child entity survives, ie no despawn-respawn churn
		world.entities().contains(generation).xpect_true();
		child_count(&mut world, list).xpect_eq(1);
	}

	/// Native events mutating a list field, no DOM and no `BlobStore`: prove the
	/// full target-agnostic loop of event then document mutation then
	/// change-detected rebuild, fully synchronously.
	#[derive(EntityTargetEvent)]
	struct PushItem;

	#[derive(EntityTargetEvent)]
	struct PopItem;

	#[beet_core::test]
	fn native_event_drives_list() {
		let mut world = DocumentPlugin::world();
		let items = TypedFieldRef::<Vec<String>>::new("items");

		// observers mutate the field through FieldQuery, no render-target coupling
		let push = items.clone();
		world.add_observer(move |ev: On<PushItem>, mut fields: FieldQuery| {
			fields
				.update_typed(ev.target(), &push, |list| {
					list.push("row".into())
				})
				.unwrap();
		});
		let pop = items.clone();
		world.add_observer(move |ev: On<PopItem>, mut fields: FieldQuery| {
			fields
				.update_typed(ev.target(), &pop, |list| {
					list.pop();
				})
				.unwrap();
		});

		let doc = world.spawn(Document::default()).id();
		// the child resolves the field through DocumentPath::Ancestor
		let list = world
			.spawn((
				ChildOf(doc),
				ReactiveChildren::new(items.field(), |_, value| {
					OnSpawn::insert(value.clone())
				}),
			))
			.id();
		world.update_local();
		child_count(&mut world, list).xpect_eq(0);

		world.entity_mut(doc).trigger_target(PushItem).flush();
		world.entity_mut(doc).trigger_target(PushItem).flush();
		world.update_local();
		child_count(&mut world, list).xpect_eq(2);

		world.entity_mut(doc).trigger_target(PopItem).flush();
		world.update_local();
		child_count(&mut world, list).xpect_eq(1);
	}

	/// Collect the [`Value`] of each leaf entity carrying a [`FieldRef`].
	fn field_values(world: &mut World) -> Vec<Value> {
		world
			.query_once::<(&Value, &FieldRef)>()
			.iter()
			.map(|(value, _)| (*value).clone())
			.collect()
	}

	#[beet_core::test]
	fn child_field_resolves_to_item() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({
				"items": [{ "name": "Alice" }, { "name": "Bob" }]
			})))
			.id();
		world.spawn((
			ChildOf(doc),
			ReactiveChildren::new(FieldRef::new("items"), |_, _| {
				// each child reads its own item's "name", scoped to items[N]
				OnSpawn::insert((Value::default(), FieldRef::new("name")))
			}),
		));
		// children spawn the first pass, their FieldRef resolves and syncs the
		// next, so a second pass settles the leaf values
		world.update_local();
		world.update_local();

		let values = field_values(&mut world);
		values.contains(&Value::Str("Alice".into())).xpect_true();
		values.contains(&Value::Str("Bob".into())).xpect_true();
	}

	#[beet_core::test]
	fn nested_list_no_double_count() {
		let mut world = DocumentPlugin::world();
		// outer list of groups, each with an inner list of items
		let doc = world
			.spawn(Document::new(val!({
				"groups": [
					{ "items": [{ "name": "a0" }, { "name": "a1" }] },
					{ "items": [{ "name": "b0" }] }
				]
			})))
			.id();
		// the outer ReactiveChildren spawns one child per group; each group child
		// hosts an inner ReactiveChildren over its own "items"
		world.spawn((
			ChildOf(doc),
			ReactiveChildren::new(FieldRef::new("groups"), |_, _| {
				OnSpawn::insert(ReactiveChildren::new(
					FieldRef::new("items"),
					|_, _| {
						OnSpawn::insert((Value::default(), FieldRef::new("name")))
					},
				))
			}),
		));
		// outer children -> outer FieldRef syncs -> inner children -> inner
		// FieldRef syncs -> leaf "name" syncs: four staged passes
		for _ in 0..4 {
			world.update_local();
		}

		// leaves resolve to groups[G].items[I].name without double-counting the
		// outer groups[G] prefix (the terminating child scope seals it)
		let values = field_values(&mut world);
		values.contains(&Value::Str("a0".into())).xpect_true();
		values.contains(&Value::Str("a1".into())).xpect_true();
		values.contains(&Value::Str("b0".into())).xpect_true();
	}
}
