use crate::prelude::*;
use anyhow::anyhow;
use anyhow::Result;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::world::World;
use petgraph::graph::DiGraph;


#[derive(Default, Clone, Deref, DerefMut)]
pub struct WillyBehavoirGraph(pub DiGraph<WillyBehaviorNode, ()>);

impl WillyBehavoirGraph {
	pub fn into_scene<T: ActionTypes>(&self) {}

	pub fn spawn(&self, world: &mut World) -> EntityGraph {
		EntityGraph::spawn_no_target(world, self.clone())
	}

	/// # Errors
	/// If a type in the graph is missing from `T`
	fn get_checked_type_registry<T: ActionTypes>(
		&self,
	) -> Result<AppTypeRegistry> {
		let registry = BehaviorGraphPrefab::<T>::get_type_registry();
		let registry_read = registry.read();
		for node in self.node_weights() {
			for action in node.actions.iter() {
				registry_read
					.get_type_data::<ReflectAction>(action.type_id())
					.ok_or_else(|| {
						anyhow::anyhow!(
							"Type not registered: {:?}",
							action.type_id()
						)
					})?;
			}
		}
		drop(registry_read);

		Ok(registry)
	}

	pub fn into_prefab<T: ActionTypes>(self) -> Result<BehaviorGraphPrefab<T>> {
		let mut world = World::new();
		let entity_graph =
			EntityGraph::spawn_no_target(&mut world, self.clone());
		let _root = entity_graph
			.root()
			.ok_or_else(|| anyhow!("No root entity"))?;
		let registry = self.get_checked_type_registry::<T>()?;
		world.insert_resource(registry);
		Ok(BehaviorGraphPrefab::from_world(world))
	}
}
