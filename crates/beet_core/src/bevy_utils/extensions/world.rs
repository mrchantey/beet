use beet_utils::utils::Tree;
use bevy::ecs::component::ComponentInfo;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use extend::ext;
/// system version
pub fn log_component_names(entity: In<Entity>, world: &mut World) {
	world.log_component_names(*entity);
}


/// common trait for 'App' and 'World'
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
			.map(|ent| {
				ent.map(|comp| self.pretty_name(comp)).collect::<Vec<_>>()
			})
			.unwrap_or_default()
	}
	fn direct_component_names_related<R: RelationshipTarget>(
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
							.map(|component| self.pretty_name(component))
							.collect::<Vec<_>>()
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default()
	}

	/// Try to get the short name of a component, otherwise return the full name.
	fn pretty_name(&self, component: &ComponentInfo) -> String {
		let id = component.type_id();
		if let Some(id) = id {
			if let Some(type_registry) =
				self.into_world().get_resource::<AppTypeRegistry>()
			{
				if let Some(info) = type_registry.read().get_type_info(id) {
					return info.ty().short_path().to_string();
				}
			}
		}
		component.name().to_string()
	}


	fn log_component_names(&self, entity: Entity) {
		let names = self.component_names_related::<Children>(entity);
		let str = names.iter_to_string_indented();
		println!("Component names for {entity}: \n{str}");
		// bevy::log::info!("Component names for {entity}: \n{str}");
	}

	fn component_names_related<R: RelationshipTarget>(
		&self,
		entity: Entity,
	) -> Tree<Vec<String>> {
		let world = self.into_world();

		fn recurse<'a, R: RelationshipTarget>(
			world: &'a World,
			entity: Entity,
			visited: &mut std::collections::HashSet<Entity>,
		) -> Tree<Vec<String>> {
			if !visited.insert(entity) {
				return Tree::default(); // Prevent cycles
			}
			// Inspect the entity itself
			let value = world
				.inspect_entity(entity)
				.map(|component_iter| {
					component_iter
						.map(|component| world.pretty_name(component))
						.collect::<Vec<_>>()
				})
				.unwrap_or_default();
			// Recurse into related entities
			let children = world
				.entity(entity)
				.get::<R>()
				.map(|related| {
					related
						.iter()
						.map(|related_entity| {
							recurse::<R>(world, related_entity, visited)
						})
						.collect::<Vec<_>>()
				})
				.unwrap_or_default();
			Tree::new_with_children(value, children)
		}

		let mut visited = std::collections::HashSet::new();
		recurse::<R>(world, entity, &mut visited)
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
		self.build_scene_with(DynamicScene::from_world(self.into_world()))
	}
	#[cfg(feature = "bevy_scene")]
	fn build_scene_with(&self, scene: DynamicScene) -> String {
		use bevy::scene::ron;
		use bevy::scene::serde::SceneSerializer;

		let world = self.into_world();
		let type_registry = world.resource::<AppTypeRegistry>();
		let type_registry = type_registry.read();
		let scene_serializer = SceneSerializer::new(&scene, &type_registry);
		let pretty_config = ron::ser::PrettyConfig::default()
			.indentor("  ".to_string())
			.new_line("\n".to_string());
		let scene =
			ron::ser::to_string_pretty(&scene_serializer, pretty_config)
				.expect("failed to serialize scene");
		scene
	}
	#[cfg(feature = "bevy_scene")]
	fn load_scene(&mut self, scene: impl AsRef<str>) -> Result {
		self.load_scene_with(scene, &mut Default::default())
	}
	#[cfg(feature = "bevy_scene")]
	fn load_scene_with(
		&mut self,
		scene: impl AsRef<str>,
		entity_map: &mut bevy::ecs::entity::EntityHashMap<Entity>,
	) -> Result {
		let scene = scene.as_ref();
		let world = self.into_world_mut();
		let scene = {
			use serde::de::DeserializeSeed;
			let type_registry = world.resource::<AppTypeRegistry>();
			let mut deserializer =
				bevy::scene::ron::de::Deserializer::from_str(scene)?;
			let scene_deserializer = bevy::scene::serde::SceneDeserializer {
				type_registry: &type_registry.read(),
			};

			scene_deserializer
				.deserialize(&mut deserializer)
				.map_err(|e| deserializer.span_error(e))
		}?;
		scene.write_to_world(world, entity_map)?;

		Ok(())
	}
}
