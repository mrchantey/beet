//! [`Prefab`]: a subtree held as a template, cloned per use.
use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::ecs::entity::EntityCloner;
use bevy::ecs::entity::EntityHashMap;

/// The root of a prefab: a subtree held as a template and cloned under each
/// use site with Bevy's entity cloning ([`clone_under`](Self::clone_under)),
/// never built again. Each use has its own entities, state and input target.
/// In markup a prefab is HTML's `<template>`.
///
/// The whole subtree is [`Held`] ([`hold`](Self::hold)), so nothing of it
/// shows or runs where it is held: no render walks it and no system binds
/// it. The marker holds its subtree once the spawn adding it settles, a
/// builder that spawns the subtree in steps holds it when done, and a body
/// load holds every prefab it spawned. A prefab that is content, the
/// `<template>` of a [`ReactiveChildren`], saves as this marker and its
/// subtree, and an authored [`Disabled`] within it saves and clones as the
/// content it is.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Held)]
#[component(on_add = hook_ext::entity_hook(|entity| {
	entity.queue(|entity: EntityWorldMut| {
		let prefab = entity.id();
		Prefab::hold(entity.into_world_mut(), prefab);
	});
}))]
pub struct Prefab;

/// Holds an entity under a [`Prefab`]: a disabling component beside Bevy's
/// [`Disabled`], so a query skips a held entity unless it names `Held`.
/// Installed by type, `app.install_type::<Held>()`, by every plugin a prefab
/// may appear under, before any query is built. Never reflected, so never
/// saved: a load holds each prefab it spawns again, and a clone never carries
/// it.
///
/// Distinct from [`Disabled`], which is content an author writes and a save
/// keeps, so `Held` never brings `Disabled` with it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component)]
pub struct Held;

/// Registers `Held` as a disabling component, idempotently.
impl TypeInstaller for Held {
	fn install(app: &mut App) {
		app.world_mut().register_disabling_component::<Self>();
	}
}

/// What [`Prefab::clone_under`] spawned.
#[derive(Debug, Clone)]
pub struct PrefabClone {
	/// The clone of the prefab's root.
	pub root: Entity,
	/// The clone's entities by key: the keys the prefab was loaded with where
	/// it retains a [`TemplateEntityMap`], else minted in tree order.
	pub keys: TemplateEntityMap,
}

impl Prefab {
	/// Clone the prefab rooted at `prefab` under `target`, its root the
	/// clone's root and every entity reference within it remapped to the
	/// clone, so two clones share nothing. The clone carries neither
	/// [`Prefab`] nor [`Held`], save a prefab within it, which stays one, and
	/// lands after `target`'s existing children.
	///
	/// `use_site` is inserted on the clone's root before anything is cloned,
	/// so every hook the clone runs already sees it, ie a row's
	/// [`DocumentScope`]. Derived state ([`ReflectDerived`]) is never cloned,
	/// since a clone recomputes its own, and a [`Derived`] entity under the
	/// prefab (the rows a nested [`ReactiveChildren`] generated) is left out,
	/// since the clone generates its own.
	pub fn clone_under(
		world: &mut World,
		prefab: Entity,
		target: Entity,
		use_site: impl Bundle,
	) -> Result<PrefabClone> {
		if !world
			.get_entity(prefab)
			.is_ok_and(|entity| entity.contains::<Self>())
		{
			bevybail!("{prefab} is no prefab");
		}
		let sources = Self::subtree(world, prefab);
		// every clone is allocated first, so a reference anywhere in the
		// subtree maps to its clone whichever order the entities clone in
		let mut map = EntityHashMap::<Entity>::default();
		for source in &sources {
			map.insert(*source, world.spawn_empty().id());
		}
		// the root's own parent stands for the use site
		let root = map[&prefab];
		match world.get::<ChildOf>(prefab).map(ChildOf::parent) {
			Some(parent) => {
				map.insert(parent, target);
				world.entity_mut(root).insert(use_site);
			}
			None => {
				world.entity_mut(root).insert((ChildOf(target), use_site));
			}
		}
		let derived = Self::derived_components(world);
		let mut cloner = |nested: bool| {
			let mut builder = EntityCloner::build_opt_out(world);
			// the hierarchy rebuilds from each clone's `ChildOf`, in tree order
			builder
				.deny::<(Children, TemplateEntityMap)>()
				.deny_by_ids(derived.iter().copied());
			if !nested {
				builder.deny::<(Self, Held)>();
			}
			builder.finish()
		};
		let (mut content, mut held) = (cloner(false), cloner(true));
		// a prefab within this one, the item of a list in a row, is content of
		// the clone and held there in turn
		let mut nested = HashSet::<Entity>::default();
		for source in &sources {
			let under_nested = world
				.get::<ChildOf>(*source)
				.is_some_and(|child| nested.contains(&child.parent()));
			let is_prefab =
				*source != prefab && world.get::<Self>(*source).is_some();
			if under_nested || is_prefab {
				nested.insert(*source);
				held.clone_entity_mapped(world, *source, &mut map);
			} else {
				content.clone_entity_mapped(world, *source, &mut map);
			}
		}
		let keys = match world.get::<TemplateEntityMap>(prefab) {
			Some(loaded) => TemplateEntityMap::from_pairs(
				sources.iter().filter_map(|source| {
					loaded
						.get_file_key(*source)
						.map(|key| (key.clone(), map[source]))
				}),
			),
			None => TemplateEntityMap::from_pairs(
				sources.iter().enumerate().map(|(index, source)| {
					(EntityKey::minted(index as u32), map[source])
				}),
			),
		};
		PrefabClone { root, keys }.xok()
	}

	/// Every component whose type is marked [`ReflectDerived`].
	fn derived_components(world: &World) -> Vec<ComponentId> {
		let Some(registry) = world.get_resource::<AppTypeRegistry>() else {
			return default();
		};
		registry
			.read()
			.iter_with_data::<ReflectDerived>()
			.filter_map(|(registration, _)| {
				world.components().get_id(registration.type_id())
			})
			.collect()
	}

	/// Hold every entity under the prefab [`Held`] beside its root, once its
	/// subtree is built, so no system binds or renders the template.
	pub fn hold(world: &mut World, prefab: Entity) {
		for entity in Self::subtree(world, prefab) {
			world.entity_mut(entity).insert(Held);
		}
	}

	/// `root` and its descendants in tree order, each [`Derived`] subtree
	/// left out.
	fn subtree(world: &World, root: Entity) -> Vec<Entity> {
		let mut found = Vec::new();
		let mut stack = vec![root];
		while let Some(entity) = stack.pop() {
			found.push(entity);
			if let Some(children) = world.get::<Children>(entity) {
				stack.extend(
					children
						.iter()
						.rev()
						.filter(|child| world.get::<Derived>(*child).is_none()),
				);
			}
		}
		found
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A component naming another entity, which a clone remaps.
	#[derive(Debug, Clone, PartialEq, Component, Reflect, MapEntities)]
	#[reflect(Component, MapEntities)]
	struct Points(#[entities] Entity);

	/// A prefab whose child points at its sibling, under a holder.
	fn prefab(world: &mut World) -> Entity {
		let holder = world.spawn_empty().id();
		let prefab = world.spawn((Prefab, ChildOf(holder))).id();
		let sibling = world.spawn((Name::new("b"), ChildOf(prefab))).id();
		world.spawn((Name::new("a"), Points(sibling), ChildOf(prefab)));
		prefab
	}

	#[crate::test]
	fn clones_are_independent_and_remapped() {
		let mut world = TemplatePlugin::world();
		let prefab = prefab(&mut world);
		Prefab::hold(&mut world, prefab);
		let (first, second) =
			(world.spawn_empty().id(), world.spawn_empty().id());
		let clones = [first, second].map(|target| {
			Prefab::clone_under(&mut world, prefab, target, ()).unwrap()
		});
		for (clone, target) in clones.iter().zip([first, second]) {
			let root = world.entity(clone.root);
			root.get::<ChildOf>().unwrap().parent().xpect_eq(target);
			root.contains::<Held>().xpect_false();
			root.contains::<Prefab>().xpect_false();
			let children = root.get::<Children>().unwrap().to_vec();
			children.len().xpect_eq(2);
			world.entity(children[0]).contains::<Held>().xpect_false();
			// the reference lands on this clone's sibling, not the prefab's
			world
				.get::<Points>(children[1])
				.unwrap()
				.0
				.xpect_eq(children[0]);
			clone.keys.world(0).unwrap().xpect_eq(clone.root);
			clone.keys.world(1).unwrap().xpect_eq(children[0]);
		}
		(clones[0].root == clones[1].root).xpect_false();
		// the prefab itself is untouched and still held
		world.entity(prefab).contains::<Held>().xpect_true();
	}

	/// An authored `Disabled` is content: a held query skips it as it skips
	/// the held subtree, and a clone keeps it where the clone drops `Held`.
	#[crate::test]
	fn held_is_not_disabled() {
		let mut world = TemplatePlugin::world();
		let prefab = world.spawn(Prefab).id();
		let off = world
			.spawn((Name::new("off"), Disabled, ChildOf(prefab)))
			.id();
		Prefab::hold(&mut world, prefab);
		world.entity(prefab).contains::<Disabled>().xpect_false();
		world.query::<&Name>().iter(&world).count().xpect_eq(0);
		world.entity(off).contains::<Held>().xpect_true();
		let target = world.spawn_empty().id();
		let clone =
			Prefab::clone_under(&mut world, prefab, target, ()).unwrap();
		let child = world.entity(clone.root).get::<Children>().unwrap()[0];
		world.entity(child).contains::<Disabled>().xpect_true();
		world.entity(child).contains::<Held>().xpect_false();
	}

	/// A row a nested generator made under the prefab is not cloned.
	#[crate::test]
	fn derived_rows_are_left_out() {
		let mut world = World::new();
		let prefab = world.spawn(Prefab).id();
		world.spawn((Name::new("row"), Derived, ChildOf(prefab)));
		let target = world.spawn_empty().id();
		let clone =
			Prefab::clone_under(&mut world, prefab, target, ()).unwrap();
		world.entity(clone.root).get::<Children>().xpect_none();
	}

	/// An authored `Disabled` is content a save keeps, where `Held` is never
	/// written and a load holds the prefab again.
	#[cfg(feature = "json")]
	#[crate::test]
	fn disabled_saves_and_held_does_not() {
		let new_world = || <(TemplatePlugin, MinimalTypesPlugin)>::world();
		let mut world = new_world();
		let root = world.spawn(Name::new("root")).id();
		world.spawn((Name::new("off"), Disabled, ChildOf(root)));
		let prefab = world.spawn((Prefab, ChildOf(root))).id();
		world.spawn((Name::new("held"), ChildOf(prefab)));
		world.flush();
		let bytes = TemplateSaver::new()
			.save_roots(&mut world, MediaType::Json, [root])
			.unwrap();
		bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains("Disabled")
			.xpect_contains("Prefab")
			.xnot()
			.xpect_contains("Held");
		let mut loaded = new_world();
		let root = TemplateLoader::new(&mut loaded).load(&bytes).unwrap()[0];
		loaded.flush();
		let children = loaded.entity(root).get::<Children>().unwrap().to_vec();
		loaded
			.entity(children[0])
			.contains::<Disabled>()
			.xpect_true();
		let held = loaded.entity(children[1]).get::<Children>().unwrap()[0];
		loaded.entity(held).contains::<Held>().xpect_true();
	}
}
