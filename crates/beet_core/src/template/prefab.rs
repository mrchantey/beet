//! [`Prefab`]: a subtree held as a template, cloned per use.
use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::ecs::entity::EntityCloner;
use bevy::ecs::entity::EntityHashMap;

/// The root of a prefab: a subtree held [`Disabled`] as a template and
/// cloned under each use site with Bevy's entity cloning
/// ([`clone_under`](Self::clone_under)), never built again. Each use has its
/// own entities, state and input target.
///
/// The whole subtree is held [`Disabled`] ([`hold`](Self::hold)), so nothing
/// of it shows or runs where it is held: no render walks it and no system
/// binds it. The marker holds its subtree once the spawn adding it settles,
/// a builder that spawns the subtree in steps holds it when done, and a body
/// load holds every prefab it spawned. `Disabled` is derived state, never
/// saved: a prefab that is content, the item template of a
/// [`ReactiveChildren`], saves as this marker and its subtree.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Disabled)]
#[component(on_add = hook_ext::entity_hook(|entity| {
	entity.queue(|entity: EntityWorldMut| {
		let prefab = entity.id();
		Prefab::hold(entity.into_world_mut(), prefab);
	});
}))]
pub struct Prefab;

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
	/// clone, so two clones share nothing. The clone is enabled, carrying
	/// neither [`Prefab`] nor [`Disabled`], and lands after `target`'s
	/// existing children.
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
		let mut cloner = {
			let mut builder = EntityCloner::build_opt_out(world);
			// the hierarchy rebuilds from each clone's `ChildOf`, in tree order
			builder
				.deny::<(Self, Disabled, Children, TemplateEntityMap)>()
				.deny_by_ids(derived);
			builder.finish()
		};
		for source in &sources {
			cloner.clone_entity_mapped(world, *source, &mut map);
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

	/// Hold every entity under the prefab [`Disabled`] beside its root, once
	/// its subtree is built, so no system binds or renders the template.
	pub fn hold(world: &mut World, prefab: Entity) {
		for entity in Self::subtree(world, prefab) {
			world.entity_mut(entity).insert(Disabled);
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
		let mut world = World::new();
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
			root.contains::<Disabled>().xpect_false();
			root.contains::<Prefab>().xpect_false();
			let children = root.get::<Children>().unwrap().to_vec();
			children.len().xpect_eq(2);
			world
				.entity(children[0])
				.contains::<Disabled>()
				.xpect_false();
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
		world.entity(prefab).contains::<Disabled>().xpect_true();
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
}
