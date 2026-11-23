use crate::prelude::*;
use bevy::ecs::query::Allow;
use bevy::platform::sync::Arc;

/// Reactive structure: one child per item of a list-typed document field,
/// reconciled whenever that field's [`Value`] changes. `bx:for` in markup
/// is sugar for it.
///
/// The companion to [`FieldRef`], which reactively syncs a single [`Value`].
/// Spawn it beside the ref backing it, `(FieldRef::new("items"),
/// ReactiveChildren::default())`: the ref links to the document and inserts
/// the synced [`Value`], so a `Changed<Value>` on this entity drives the
/// rebuild.
///
/// Each row is a clone of the item template, a [`Prefab`] child of this
/// entity, in markup its `<template>` ([`Prefab::clone_under`]), in a
/// terminating [`DocumentScope`] at its item, so a binding in the template
/// reads its own item. A row cloned from a `<template>` element is its
/// content and never a `<template>` itself. Rows are [`Derived`], so a save writes the generator and its
/// template and never the rows, which the next load regenerates. With a
/// [`key`](Self::key) a change reconciles by key rather than rebuilding:
/// matching rows are reused, removed keys despawned and new keys spawned, so
/// per-row state and bindings survive an append.
///
/// A view built in rust whose rows are computed per item, rather than bound,
/// gives a builder instead of a prefab ([`from_fn`](Self::from_fn)). A builder
/// is code, which a scene cannot hold, so such a view is derived ui and never
/// saved.
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ReactiveChildren {
	/// The field of each item a row reconciles by, ie `id` or `user.id`
	/// (`bx:key`); a non-string key compares by its debug form. Unkeyed, a
	/// change rebuilds every row.
	pub key: Option<FieldPath>,
	/// Builds a row's content from its index and item, in place of the
	/// prefab.
	#[reflect(ignore)]
	build: Option<ItemBuilder>,
}

/// A row builder, see [`ReactiveChildren::from_fn`].
#[derive(Clone)]
struct ItemBuilder(Arc<dyn Fn(usize, &Value) -> OnSpawn + Send + Sync>);

/// Marker on each child spawned by a [`ReactiveChildren`], so a rebuild can
/// despawn exactly the previous generation and leave static siblings alone.
#[derive(Component)]
pub struct ReactiveChild;

/// The reconciliation key and current index of a keyed [`ReactiveChild`].
#[derive(Component)]
pub(crate) struct ReactiveChildKey {
	key: String,
	index: usize,
}

impl ReactiveChildren {
	/// Rows built per item by `build_item` rather than cloned from a prefab,
	/// for a view built in rust.
	pub fn from_fn(
		build_item: impl 'static + Send + Sync + Fn(usize, &Value) -> OnSpawn,
	) -> Self {
		Self {
			key: None,
			build: Some(ItemBuilder(Arc::new(build_item))),
		}
	}

	/// Reconcile rows by the item field at `key`.
	pub fn with_key(mut self, key: impl Into<FieldPath>) -> Self {
		self.key = Some(key.into());
		self
	}

	/// The reconciliation key of `item`: the string at [`key`](Self::key),
	/// else its debug form, empty where the item lacks it.
	fn item_key(key: &FieldPath, item: &Value) -> String {
		match item.get_path(key) {
			Some(Value::Str(key)) => key.to_string(),
			Some(other) => format!("{other:?}"),
			None => String::new(),
		}
	}

	/// Queue the spawn of one row under `parent`, carrying `row`: built by
	/// the builder, else cloned from `prefab`.
	fn spawn_row(
		&self,
		commands: &mut Commands,
		parent: Entity,
		prefab: Option<Entity>,
		index: usize,
		item: &Value,
		row: impl Bundle,
	) {
		let row = (ChildOf(parent), ReactiveChild, Derived, row);
		match (&self.build, prefab) {
			(Some(build), _) => {
				commands.spawn((row, (build.0)(index, item)));
			}
			(None, Some(prefab)) => {
				commands.queue(move |world: &mut World| {
					match Prefab::clone_under(world, prefab, parent, row) {
						// a row cloned from a `<template>` is its content, as
						// cloning HTML's `template.content` is, never a
						// `<template>` itself
						Ok(clone) => {
							if world.get::<Element>(clone.root).is_some_and(
								|element| element.tag() == "template",
							) {
								world
									.entity_mut(clone.root)
									.remove::<Element>();
							}
						}
						Err(err) => {
							world.handle_command_error::<ReactiveChildren>(err);
						}
					}
				});
			}
			(None, None) => {}
		}
	}
}

/// System that rebuilds [`ReactiveChildren`] when their synced [`Value`] changes.
///
/// Runs in [`DocumentSyncSet::Rebuild`]: after
/// [`sync_document_to_local`](super::sync_document_to_local) writes the
/// [`Value`] and marks it `Changed`, so the rebuild reads the current list the
/// same pass (including the initial generation), and before the write-back, so
/// a vanished item's children never write into a list that has dropped them.
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
	prefabs: Query<(), (With<Prefab>, Allow<Held>)>,
	child_keys: Query<&ReactiveChildKey>,
) -> Result {
	for (entity, value, resolved, reactive, children) in changed.iter() {
		// the child's fully-resolved absolute item path, terminating so an inner
		// FieldRef does not double-count outer scopes
		let scope_at = |index: usize| DocumentScope {
			path: resolved.field_path.with_pushed(index),
			terminate: true,
		};
		let prefab = children
			.into_iter()
			.flatten()
			.copied()
			.find(|child| prefabs.contains(*child));
		let items = value.as_list().map(Vec::as_slice).unwrap_or_default();
		match &reactive.key {
			// unkeyed: despawn the previous generation and respawn it whole
			None => {
				for child in children
					.into_iter()
					.flatten()
					.filter(|child| reactive_children.contains(**child))
				{
					commands.entity(*child).despawn();
				}
				for (index, item) in items.iter().enumerate() {
					reactive.spawn_row(
						&mut commands,
						entity,
						prefab,
						index,
						item,
						scope_at(index),
					);
				}
			}
			// keyed: reconcile by key, reusing matching children
			Some(key_path) => {
				let existing: HashMap<String, Entity> = children
					.into_iter()
					.flatten()
					.filter_map(|child| {
						child_keys
							.get(*child)
							.ok()
							.map(|keyed| (keyed.key.clone(), *child))
					})
					.collect();
				let mut kept: HashSet<String> = HashSet::default();
				for (index, item) in items.iter().enumerate() {
					let key = ReactiveChildren::item_key(key_path, item);
					kept.insert(key.clone());
					match existing.get(&key) {
						// reuse: only touch the child if its index shifted, so
						// an append never re-resolves a settled row's bindings
						Some(&child) => {
							let unchanged = child_keys
								.get(child)
								.map(|keyed| keyed.index)
								.unwrap_or(usize::MAX) == index;
							if !unchanged {
								commands.entity(child).insert((
									scope_at(index),
									ReactiveChildKey { key, index },
								));
							}
						}
						None => reactive.spawn_row(
							&mut commands,
							entity,
							prefab,
							index,
							item,
							(ReactiveChildKey { key, index }, scope_at(index)),
						),
					}
				}
				// despawn children whose key vanished
				for (_, child) in
					existing.iter().filter(|(key, _)| !kept.contains(*key))
				{
					commands.entity(*child).despawn();
				}
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
	#[crate::test]
	fn keyed_reuses_children_across_append_and_remove() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(value!({ "items": ["a", "b"] })))
			.id();
		let list = world
			.spawn((
				ChildOf(doc),
				(
					FieldRef::new("items"),
					ReactiveChildren::from_fn(|_, value| {
						OnSpawn::insert(value.clone())
					})
					.with_key(FieldPath::default()),
				),
			))
			.id();
		world.update_local();
		let generation_1 = reactive_children_of(&mut world, list);
		generation_1.len().xpect_eq(2);

		// append "c": the two existing rows are reused (same entities), in order
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			value!({ "items": ["a", "b", "c"] });
		world.update_local();
		let generation_2 = reactive_children_of(&mut world, list);
		generation_2.len().xpect_eq(3);
		generation_2[0].xpect_eq(generation_1[0]);
		generation_2[1].xpect_eq(generation_1[1]);

		// remove "b": only its entity is despawned, "a" and "c" survive
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			value!({ "items": ["a", "c"] });
		world.update_local();
		let generation_3 = reactive_children_of(&mut world, list);
		generation_3.len().xpect_eq(2);
		generation_3[0].xpect_eq(generation_1[0]);
		generation_3[1].xpect_eq(generation_2[2]);
		world.entities().contains(generation_1[1]).xpect_false();
	}

	#[crate::test]
	fn grows_and_shrinks() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(value!({ "items": ["a", "b"] })))
			.id();
		let list = world
			.spawn((
				ChildOf(doc),
				(
					FieldRef::new("items"),
					ReactiveChildren::from_fn(|_, value| {
						OnSpawn::insert(value.clone())
					}),
				),
			))
			.id();
		world.update_local();
		child_count(&mut world, list).xpect_eq(2);

		// grow
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			value!({ "items": ["a", "b", "c"] });
		world.update_local();
		child_count(&mut world, list).xpect_eq(3);

		// shrink
		world.entity_mut(doc).get_mut::<Document>().unwrap().0 =
			value!({ "items": ["a"] });
		world.update_local();
		child_count(&mut world, list).xpect_eq(1);
	}

	#[crate::test]
	fn leaves_static_siblings_untouched() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(value!({ "items": ["a", "b"] })))
			.id();
		let list = world
			.spawn((
				ChildOf(doc),
				(
					FieldRef::new("items"),
					ReactiveChildren::from_fn(|_, value| {
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
			value!({ "items": [] });
		world.update_local();
		// reactive children despawned, static sibling survives
		child_count(&mut world, list).xpect_eq(0);
		world.entities().contains(static_child).xpect_true();
	}

	#[crate::test]
	fn rebuilds_only_on_own_value_change() {
		let mut world = DocumentPlugin::world();
		let doc = world.spawn(Document::new(value!({ "items": ["a"] }))).id();
		let list = world
			.spawn((
				ChildOf(doc),
				(
					FieldRef::new("items"),
					ReactiveChildren::from_fn(|_, value| {
						OnSpawn::insert(value.clone())
					}),
				),
			))
			.id();
		world.update_local();
		child_count(&mut world, list).xpect_eq(1);
		let generation = world.entity(list).get::<Children>().unwrap()[0];

		// an unrelated document changing must not rebuild this list
		world.spawn(Document::new(value!({ "other": 1i64 })));
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

	#[crate::test]
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
					ReactiveChildren::from_fn(|_, value| {
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

	#[crate::test]
	fn child_field_resolves_to_item() {
		let mut world = DocumentPlugin::world();
		let doc = world
			.spawn(Document::new(value!({
				"items": [{ "name": "Alice" }, { "name": "Bob" }]
			})))
			.id();
		world.spawn((
			ChildOf(doc),
			(
				FieldRef::new("items"),
				ReactiveChildren::from_fn(|_, _| {
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

	#[crate::test]
	fn nested_list_no_double_count() {
		let mut world = DocumentPlugin::world();
		// outer list of groups, each with an inner list of items
		let doc = world
			.spawn(Document::new(value!({
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
				ReactiveChildren::from_fn(|_, _| {
					OnSpawn::insert((
						FieldRef::new("items"),
						ReactiveChildren::from_fn(|_, _| {
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
