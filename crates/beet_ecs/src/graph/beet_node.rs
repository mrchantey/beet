use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;
use bevy::utils::HashSet;

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

pub type SpawnFunc = Box<dyn FnOnce(&mut World) -> Entity>;

pub trait BeetBundle: Bundle + Reflect + GetTypeRegistration {}
impl<T: Bundle + Reflect + GetTypeRegistration> BeetBundle for T {}

/// An opaque intermediary structure between a [`Bundle`] graph and a [`BehaviorPrefab`]
/// This does the following when build
/// - Registers the bundle types
/// - Spawns the entities
/// - maps children to an [`Edges`] component
pub struct BeetNode {
	pub children: Vec<BeetNode>,
	pub spawn_func: SpawnFunc,
	// great name buddy
	pub misc_funcs: Vec<Box<dyn FnOnce(&mut World)>>,
	// pub world: Arc<RwLock<World>>,
}

impl BeetNode {
	pub fn new<T: BeetBundle>(bundle: T) -> Self {
		Self {
			children: Vec::new(),
			spawn_func: Box::new(move |world: &mut World| {
				Self::register_type::<T>(world);
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

	pub fn spawn(self, world: &mut World, agent: Entity) -> EntityTree {
		let tree = self.spawn_no_target(world);
		tree.bind_agent(world, agent);
		tree
	}

	pub fn spawn_no_target(self, world: &mut World) -> EntityTree {
		let tree = self.build_recursive(world, &mut HashSet::default());
		world
			.entity_mut(tree.value)
			.insert((BehaviorGraphRoot, Running));
		EntityTree(tree)
	}


	fn build_recursive(
		self,
		world: &mut World,
		visited: &mut HashSet<Entity>,
	) -> Tree<Entity> {
		for func in self.misc_funcs {
			func(world);
		}
		let entity = (self.spawn_func)(world);
		visited.insert(entity);

		let children = self
			.children
			.into_iter()
			.map(|child| child.build_recursive(world, visited))
			.collect::<Vec<_>>();

		let edges = Edges(children.iter().map(|c| c.value).collect());

		let mut entity = world.entity_mut(entity);
		if false == entity.contains::<Name>() {
			let id = visited.len();
			entity.insert(Name::new(format!("Node {id}")));
		}
		entity.insert((RunTimer::default(), edges));

		Tree {
			value: entity.id(),
			children,
		}
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
	fn into_prefab(self) -> Result<BehaviorPrefab> {
		let mut world = World::new();
		let tree = self.into_beet_node().spawn_no_target(&mut world);
		Ok(BehaviorPrefab::from_world(&mut world, tree.value))
	}
}
