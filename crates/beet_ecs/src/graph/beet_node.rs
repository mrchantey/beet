use crate::prelude::*;
use anyhow::Result;
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_reflect::GetTypeRegistration;
use bevy_reflect::Reflect;
use bevy_utils::HashSet;

/// Marker to identify the root of a behavior graph
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct BehaviorGraphRoot;

// pub struct EntityGraphOptions {
// 	agent: Option<Entity>,
// 	run_on_spawn: bool,
// }

// impl Default for EntityGraphOptions {
// 	fn default() -> Self {
// 		Self {
// 			agent: None,
// 			run_on_spawn: true,
// 		}
// 	}
// }

const EXPECT_OK: &str =
	"Unexpected Error, spawning from beet node should always be safe";

pub type SpawnFunc = Box<dyn FnOnce(&mut World) -> Entity>;

pub trait BeetBundle: Bundle + Reflect + GetTypeRegistration {}
impl<T: Bundle + Reflect + GetTypeRegistration> BeetBundle for T {}

pub struct BeetNode {
	pub children: Vec<BeetNode>,
	pub spawn_func: SpawnFunc,
	pub misc_funcs: Vec<Box<dyn FnOnce(&mut World)>>,
	// pub world: Arc<RwLock<World>>,
}

impl BeetNode {
	pub fn new<T: BeetBundle>(bundle: T) -> Self {
		Self {
			children: Vec::new(),
			spawn_func: Box::new(move |world: &mut World| {
				Self::register_type::<T>(world);

				// let type_data = T::get_type_registration();
				// type_data.

				world.spawn(bundle).id()
			}),
			misc_funcs: Vec::new(),
		}
	}
	pub fn with_type<T: GetTypeRegistration>(mut self) -> Self {
		self.misc_funcs.push(Box::new(|world: &mut World| {
			Self::register_type::<T>(world);
		}));
		self
	}

	fn register_type<T: GetTypeRegistration>(world: &mut World) {
		world.init_resource::<AppTypeRegistry>();
		world
			.resource_mut::<AppTypeRegistry>()
			.write()
			.register::<T>();
	}


	/// 
	pub fn spawn<T: ActionTypes>(
		self,
		world: &mut impl IntoWorld,
		target: Entity,
	) -> EntityGraph {
		self.into_prefab::<T>()
			.expect(EXPECT_OK)
			.spawn(world, Some(target))
			.expect(EXPECT_OK)
	}
	pub fn spawn_no_target<T: ActionTypes>(
		self,
		world: &mut impl IntoWorld,
	) -> EntityGraph {
		self.into_prefab::<T>()
			.expect(EXPECT_OK)
			.spawn(world, None)
			.expect(EXPECT_OK)
	}

	pub fn build(self) -> World {
		let mut world = World::new();
		let root = self.build_recursive(&mut world, &mut HashSet::default());
		world.entity_mut(root).insert((BehaviorGraphRoot, Running));
		world
	}


	fn build_recursive(
		self,
		world: &mut World,
		visited: &mut HashSet<Entity>,
	) -> Entity {
		for func in self.misc_funcs {
			func(world);
		}
		let entity = (self.spawn_func)(world);
		visited.insert(entity);
		let id = visited.len();

		let edges = self
			.children
			.into_iter()
			.map(|child| child.build_recursive(world, visited))
			.collect();

		world
			.entity_mut(entity)
			.insert((
				Name::new(format!("Node {id}")),
				RunTimer::default(),
				Edges(edges),
			))
			.id()
	}

	pub fn child<M>(mut self, child: impl IntoBeetNode<M>) -> Self {
		self.children.push(child.into_beet_node());
		self
	}
}

pub struct IntoIntoBeetNode;
pub struct ItemIntoBeetNode;
pub struct TupleIntoBeetNode;

pub trait IntoBeetNode<M>: Sized {
	fn into_beet_node(self) -> BeetNode;
	fn child<M2>(self, child: impl IntoBeetNode<M2>) -> BeetNode {
		self.into_beet_node().child(child)
	}
}

impl<T> IntoBeetNode<IntoIntoBeetNode> for T
where
	T: Into<BeetNode>,
{
	fn into_beet_node(self) -> BeetNode { self.into() }
}

impl<T: BeetBundle> IntoBeetNode<ItemIntoBeetNode> for T {
	fn into_beet_node(self) -> BeetNode { BeetNode::new(self) }
}

pub struct BeetNodeIntoPrefab;

impl<T, M> IntoBehaviorPrefab<(BeetNodeIntoPrefab, M)> for T
where
	T: IntoBeetNode<M>,
{
	fn into_prefab<Actions: ActionTypes>(
		self,
	) -> Result<BehaviorPrefab<Actions>> {
		Ok(BehaviorPrefab::from_world(self.into_beet_node().build()))
	}
}
