use crate::prelude::*;
use anyhow::Result;
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_reflect::GetTypeRegistration;
use bevy_reflect::Reflect;
use bevy_utils::HashSet;
use std::sync::Arc;
use std::sync::RwLock;

pub trait BeetBundle: Bundle + Reflect {}
impl<T: Bundle + Reflect> BeetBundle for T {}

pub struct BeetNode {
	pub entity: Entity,
	pub children: Vec<BeetNode>,
	pub world: Arc<RwLock<World>>,
}

impl BeetNode {
	pub fn new<M>(root: impl IntoBeetNode<M>) -> Self {
		root.into_beet_node(Arc::new(RwLock::new(World::default())))
	}

	/// Append a type to the world's type registry
	/// This is for components that will not appear in the action type list.
	pub fn with_type<T: GetTypeRegistration>(self) -> Self {
		let mut world = self.world.write().unwrap();
		world.init_resource::<AppTypeRegistry>();
		let registry = world.resource::<AppTypeRegistry>();
		registry.write().register::<T>();
		drop(world);
		self
	}

	pub fn with_entity_and_world(
		entity: Entity,
		world: Arc<RwLock<World>>,
	) -> Self {
		Self {
			world,
			entity,
			children: Vec::new(),
		}
	}

	fn build_recursive(&self, visited: &mut HashSet<Entity>) {
		if visited.contains(&self.entity) {
			return;
		}
		visited.insert(self.entity);
		let mut world = self.world.write().unwrap();
		let mut entity = world.entity_mut(self.entity);
		entity.insert((
			Name::new(format!("Node {}", visited.len())),
			RunTimer::default(),
		));

		let edges = self.children.iter().map(|child| child.entity).collect();
		entity.insert(Edges(edges));
		drop(entity);
		drop(world);
		for child in self.children.iter() {
			child.build_recursive(visited);
		}
	}

	pub fn child<M>(mut self, child: impl IntoBeetNode<M>) -> Self {
		let child = child.into_beet_node(self.world.clone());
		self.children.push(child);
		self
	}
}

pub struct ItemIntoBeetNode;
pub struct TupleIntoBeetNode;

pub trait IntoBeetNode<M>: Sized {
	fn into_beet_node(self, world: Arc<RwLock<World>>) -> BeetNode;
	fn child2<M2>(self, child: impl IntoBeetNode<M2>) -> BeetNode {
		let node = self.into_beet_node(Default::default());
		node.child(child)
	}
}

impl<T0: BeetBundle> IntoBeetNode<ItemIntoBeetNode> for T0 {
	#[allow(unused_variables, unused_mut)]
	fn into_beet_node(self, world: Arc<RwLock<World>>) -> BeetNode {
		let entity = world.write().unwrap().spawn(self).id();
		BeetNode::with_entity_and_world(entity, world)
	}
}

pub struct BeetNodeIntoPrefab;


impl IntoBehaviorPrefab<BeetNodeIntoPrefab> for BeetNode {
	fn into_prefab<Actions: ActionTypes>(
		self,
	) -> Result<BehaviorPrefab<Actions>> {
		self.build_recursive(&mut HashSet::default());
		let mut world = self.world.write().unwrap();
		world
			.entity_mut(self.entity)
			.insert((BehaviorGraphRoot, Running));
		let world = std::mem::take(&mut *world);
		Ok(BehaviorPrefab::from_world(world))
	}
}
