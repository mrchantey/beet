use crate::prelude::*;
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
	/// Optional stable key per item. When set, a change reconciles by key
	/// (reuse-matching / despawn-removed / spawn-new) instead of rebuilding
	/// every child, so per-row entity state and bindings survive an append.
	key_of: Option<Arc<dyn Fn(&Value) -> String + Send + Sync>>,
}

/// Marker on each child spawned by a [`ReactiveChildren`], so a rebuild can
/// despawn exactly the previous generation and leave static siblings alone.
#[derive(Component)]
pub struct ReactiveChild;

/// The reconciliation key and current index of a keyed [`ReactiveChild`].
#[derive(Component)]
pub struct ReactiveChildKey {
	key: String,
	index: usize,
}

impl ReactiveChildren {
	/// Build a [`ReactiveChildren`] that spawns a child per list item via
	/// `build_item`, rebuilding the whole generation on every change.
	///
	/// Spawn it beside the field that backs it, ie `(FieldRef::new("items"),
	/// ReactiveChildren::new(..))` or `(items.field(), ReactiveChildren::new(..))`.
	/// The ref's `on_add` inserts the synced [`Value`] and `sync_document_to_local`
	/// keeps it current, so the rebuild rides `Changed<Value>`.
	pub fn new(
		build_item: impl 'static + Send + Sync + Fn(usize, &Value) -> OnSpawn,
	) -> Self {
		ReactiveChildren {
			build_item: Arc::new(build_item),
			key_of: None,
		}
	}

	/// As [`new`](Self::new), but reconciles by the stable key `key_of` returns
	/// for each item: children whose key persists are reused (their entity,
	/// state, and field bindings survive), removed keys are despawned, and new
	/// keys are spawned. Appending an item therefore neither rebuilds existing
	/// rows nor drops their in-progress streaming bindings.
	pub fn keyed(
		key_of: impl 'static + Send + Sync + Fn(&Value) -> String,
		build_item: impl 'static + Send + Sync + Fn(usize, &Value) -> OnSpawn,
	) -> Self {
		ReactiveChildren {
			build_item: Arc::new(build_item),
			key_of: Some(Arc::new(key_of)),
		}
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
	child_keys: Query<&ReactiveChildKey>,
) -> Result {
	for (entity, value, resolved, reactive, children) in changed.iter() {
		// the child's fully-resolved absolute item path, terminating so an inner
		// FieldRef does not double-count outer scopes
		let scope_at = |index: usize| DocumentScope {
			path: resolved.field_path.with_pushed(index),
			terminate: true,
		};
		match &reactive.key_of {
			// unkeyed: despawn the previous generation and respawn it whole
			None => {
				if let Some(children) = children {
					children
						.iter()
						.filter(|child| reactive_children.contains(*child))
						.for_each(|child| commands.entity(child).despawn());
				}
				if let Ok(items) = value.as_list() {
					for (index, item) in items.iter().enumerate() {
						commands.spawn((
							ChildOf(entity),
							ReactiveChild,
							scope_at(index),
							(reactive.build_item)(index, item),
						));
					}
				}
			}
			// keyed: reconcile by key, reusing matching children
			Some(key_of) => {
				let existing: HashMap<String, Entity> = children
					.into_iter()
					.flat_map(|children| children.iter())
					.filter_map(|child| {
						child_keys
							.get(child)
							.ok()
							.map(|keyed| (keyed.key.clone(), child))
					})
					.collect();
				let mut kept: HashSet<String> = HashSet::default();
				if let Ok(items) = value.as_list() {
					for (index, item) in items.iter().enumerate() {
						let key = key_of(item);
						kept.insert(key.clone());
						match existing.get(&key) {
							// reuse: only touch the child if its index shifted, so
							// an append never re-resolves a settled row's bindings
							Some(&child) => {
								let unchanged = child_keys
									.get(child)
									.map(|keyed| keyed.index)
									.unwrap_or(usize::MAX)
									== index;
								if !unchanged {
									commands.entity(child).insert((
										scope_at(index),
										ReactiveChildKey { key, index },
									));
								}
							}
							None => {
								commands.spawn((
									ChildOf(entity),
									ReactiveChild,
									ReactiveChildKey { key, index },
									scope_at(index),
									(reactive.build_item)(index, item),
								));
							}
						}
					}
				}
				// despawn children whose key vanished
				existing
					.iter()
					.filter(|(key, _)| !kept.contains(*key))
					.for_each(|(_, child)| commands.entity(*child).despawn());
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
		reactive_children_of(world, entity).len()
	}

	/// The `ReactiveChild` children of `entity`, in `Children` order.
	fn reactive_children_of(world: &mut World, entity: Entity) -> Vec<Entity> {
		world
			.entity(entity)
			.get::<Children>()
			.map(|children| children.iter().collect::<Vec<_>>())
			.unwrap_or_default()
			.into_iter()
			.filter(|child| world.entity(*child).contains::<ReactiveChild>())
			.collect()
	}

	/// Keyed reconciliation reuses children by key: an append keeps the existing
	/// row entities (so their state and bindings survive), and a removal despawns
	/// only the vanished key.
	#[beet_core::test]
	fn keyed_reuses_children_across_append_and_remove() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(val!({ "items": ["a", "b"] })))
			.id();
		let list = world
			.spawn((
				ChildOf(doc),
				(
					FieldRef::new("items"),
					ReactiveChildren::keyed(
						|item| item.as_str().unwrap_or_default().to_string(),
						|_, value| OnSpawn::insert(value.clone()),
					),
				),
			))
			.id();
		world.update_local();
		let generation_1 = reactive_children_of(&mut world, list);
		generation_1.len().xpect_eq(2);

		// append "c": the two existing rows are reused (same entities), in order
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "items": ["a", "b", "c"] });
		world.update_local();
		let generation_2 = reactive_children_of(&mut world, list);
		generation_2.len().xpect_eq(3);
		generation_2[0].xpect_eq(generation_1[0]);
		generation_2[1].xpect_eq(generation_1[1]);

		// remove "b": only its entity is despawned, "a" and "c" survive
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			val!({ "items": ["a", "c"] });
		world.update_local();
		let generation_3 = reactive_children_of(&mut world, list);
		generation_3.len().xpect_eq(2);
		generation_3[0].xpect_eq(generation_1[0]);
		generation_3[1].xpect_eq(generation_2[2]);
		world.entities().contains(generation_1[1]).xpect_false();
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
				(
					FieldRef::new("items"),
					ReactiveChildren::new(|_, value| {
						OnSpawn::insert(value.clone())
					}),
				),
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
				(
					FieldRef::new("items"),
					ReactiveChildren::new(|_, value| {
						OnSpawn::insert(value.clone())
					}),
				),
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
		let doc = world.spawn(Document::new(val!({ "items": ["a"] }))).id();
		let list = world
			.spawn((
				ChildOf(doc),
				(
					FieldRef::new("items"),
					ReactiveChildren::new(|_, value| {
						OnSpawn::insert(value.clone())
					}),
				),
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
				(
					items.field(),
					ReactiveChildren::new(|_, value| {
						OnSpawn::insert(value.clone())
					}),
				),
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
			(
				FieldRef::new("items"),
				ReactiveChildren::new(|_, _| {
					// each child reads its own item's "name", scoped to items[N]
					OnSpawn::insert((Value::default(), FieldRef::new("name")))
				}),
			),
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
			(
				FieldRef::new("groups"),
				ReactiveChildren::new(|_, _| {
					OnSpawn::insert((
						FieldRef::new("items"),
						ReactiveChildren::new(|_, _| {
							OnSpawn::insert((
								Value::default(),
								FieldRef::new("name"),
							))
						}),
					))
				}),
			),
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
