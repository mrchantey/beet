use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy::reflect::TypeRegistryArc;
use std::sync::Arc;
use std::sync::RwLock;

/// Clones a world by serializing the scene and cloning the route handlers.
pub struct CloneWorld {
	scene: String,
	registry: TypeRegistry,
	// route handlers cannot be serialized so we clone them separately
	handlers: Vec<(Entity, RouteHandler)>,
}

impl Clone for CloneWorld {
	fn clone(&self) -> Self {
		Self {
			scene: self.scene.clone(),
			registry: clone_registry(&self.registry),
			handlers: self.handlers.clone(),
		}
	}
}

impl CloneWorld {
	pub fn new(world: &mut World) -> Self {
		let scene = world.build_scene();

		let handlers = world
			.query::<(Entity, &RouteHandler)>()
			.iter(world)
			.map(|(entity, handler)| (entity, handler.clone()))
			.collect();

		let registry =
			clone_registry(&world.resource::<AppTypeRegistry>().0.read());
		Self {
			scene,
			handlers,
			registry,
		}
	}

	pub fn clone_world(self) -> Result<World> {
		let mut app = App::new();
		app.add_plugins(RouterPlugin);
		let mut world = std::mem::take(app.world_mut());

		// let mut world = World::new();
		world.insert_resource(AppTypeRegistry(TypeRegistryArc {
			internal: Arc::new(RwLock::new(self.registry)),
		}));
		let mut entity_map = Default::default();
		world.load_scene_with(&self.scene, &mut entity_map)?;

		for (entity, handler) in self.handlers {
			let target_entity = entity_map.get(&entity).ok_or_else(|| {
				bevyhow!("Entity {} not found in cloned world", entity)
			})?;
			world.entity_mut(*target_entity).insert(handler);
		}
		Ok(world)
	}
}


fn clone_registry(registry: &TypeRegistry) -> TypeRegistry {
	let mut new_registry = TypeRegistry::default();
	for item in registry.iter() {
		new_registry.add_registration(item.clone());
	}
	new_registry
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		#[derive(Component, Reflect)]
		#[reflect(Component)]
		struct Foo(u32);

		let mut world1 = World::new();
		world1.init_resource::<AppTypeRegistry>();
		let registry = world1.resource_mut::<AppTypeRegistry>();
		registry.write().register::<Foo>();

		world1.spawn((Foo(7), RouteHandler::endpoint(|| "hello world!")));
		let mut world2 = CloneWorld::new(&mut world1).clone_world().unwrap();

		world2.query_once::<&Foo>()[0].0.xpect_eq(7);
		// Router::oneshot_str(&mut world2, "/")
		// 	.await
		// 	.unwrap()
		// 	.xpect_eq("hello world!");
	}
}
