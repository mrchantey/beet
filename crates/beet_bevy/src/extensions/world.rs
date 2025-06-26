use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use extend::ext;

trait IntoWorld {
	#[allow(unused)]
	fn into_world(&self) -> &World;
	fn into_world_mut(&mut self) -> &mut World;
}
impl IntoWorld for World {
	fn into_world(&self) -> &World { self }
	fn into_world_mut(&mut self) -> &mut World { self }
}
impl IntoWorld for App {
	fn into_world(&self) -> &World { self.world() }
	fn into_world_mut(&mut self) -> &mut World { self.world_mut() }
}

#[ext(name=WorldMutExt)]
/// Matcher extensions for `bevy::World`
pub impl<W: IntoWorld> W {
	fn component_names(&self, entity: Entity) -> Vec<String> {
		let world = self.into_world();
		world
			.inspect_entity(entity)
			.map(|e| e.map(|c| c.name().to_string()).collect::<Vec<_>>())
			.unwrap_or_default()
	}
	fn component_names_related<R: RelationshipTarget>(
		&self,
		entity: Entity,
	) -> Vec<Vec<String>> {
		let world = self.into_world();
		world
			.entity(entity)
			.get::<R>()
			.map(|related| {
				related
					.iter()
					.filter_map(|entity| world.inspect_entity(entity).ok())
					.map(|component_iter| {
						component_iter
							.map(|component| component.name().to_string())
							.collect::<Vec<_>>()
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default()
	}


	/// Shorthand for creating a query and immediatly collecting it into a Vec.
	/// This is less efficient than caching the [`QueryState`] so should only be
	/// used for one-off queries, otherwise [`World::query`] should be preferred.
	fn query_once<'a, D: QueryData>(&'a mut self) -> Vec<D::Item<'a>> {
		let world = self.into_world_mut();
		world.query::<D>().iter_mut(world).collect::<Vec<_>>()
	}

	/// Shorthand for creating a query and immediatly collecting it into a Vec.
	/// This is less efficient than caching the [`QueryState`] so should only be
	/// used for one-off queries, otherwise [`World::query_filtered`] should be preferred.
	fn query_filtered_once<'a, D: QueryData, F: QueryFilter>(
		&'a mut self,
	) -> Vec<D::Item<'a>> {
		let world = self.into_world_mut();
		world
			.query_filtered::<D, F>()
			.iter_mut(world)
			.collect::<Vec<_>>()
	}

	/// Shorthand for removing all components of a given type.
	fn remove<C: Component>(&mut self) -> Vec<C> {
		let world = self.into_world_mut();
		world
			.query_filtered::<Entity, With<C>>()
			.iter(world)
			.collect::<Vec<_>>()
			.into_iter()
			.filter_map(|entity| world.entity_mut(entity).take::<C>())
			.collect()
	}

	/// Shorthand for building a serialized scene from the current world.
	#[cfg(feature = "bevy_scene")]
	fn build_scene(&self) -> String {
		let world = self.into_world();
		let scene = DynamicScene::from_world(world);
		let type_registry = world.resource::<AppTypeRegistry>();
		let type_registry = type_registry.read();
		let scene = scene
			.serialize(&type_registry)
			.expect("failed to serialize scene");
		scene
	}
}
