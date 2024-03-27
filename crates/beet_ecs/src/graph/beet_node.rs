use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::GetTypeRegistration;
use bevy::utils::HashSet;
use std::fmt;

/// Marker to identify the root of a behavior graph
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct BehaviorGraphRoot;


/// Temporary name holder, it seems theres a bug with bevy [`Name`], cow and reflect
#[derive(Debug, Clone, Default, Component, Reflect, PartialEq)]
#[reflect(Component)]
pub struct NodeName(pub String);

impl NodeName {
	pub fn new(name: impl Into<String>) -> Self { Self(name.into()) }
}

impl fmt::Display for NodeName {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

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

/// An opaque intermediary structure between a [`Bundle`] graph and a [`DynGraph`]
/// This does the following when build
/// - Registers the bundle types
/// - Spawns the entities
/// - maps children to an [`Edges`] component
pub struct BeetNode {
	pub children: Vec<BeetNode>,
	/// Inserts [`(Running, BehaviorGraphRoot)`] components to this node if its the root
	pub insert_root_defaults: bool,
	/// Inserts [`(Name, NodeName, RunTimer)`] components to this node
	pub insert_defaults: bool,
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
			insert_root_defaults: true,
			insert_defaults: true,
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
		let insert_root_defaults = self.insert_root_defaults;
		let tree = self.build_recursive(world, &mut HashSet::default());
		if insert_root_defaults {
			world
				.entity_mut(tree.value)
				.insert((BehaviorGraphRoot, Running));
		}
		EntityTree(tree)
	}

	pub fn into_graph<T: ActionList>(self) -> DynGraph {
		DynGraph::new::<T>(self)
	}

	// TODO deprecate this in favor of an optional bundle
	pub fn insert_default_components(
		entity: &mut EntityWorldMut,
		default_name: String,
	) {
		let name = entity
			.get::<Name>()
			.map(|n| n.to_string())
			.unwrap_or(default_name);

		entity.insert((
			Name::new(name.clone()),
			NodeName::new(name),
			RunTimer::default(),
		));
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

		let mut entity = world.entity_mut(entity);

		if children.len() > 0 {
			entity.insert(Edges(children.iter().map(|c| c.value).collect()));
		}
		if self.insert_defaults {
			let default_name = format!("Node {}", visited.len());
			Self::insert_default_components(&mut entity, default_name);
		}
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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[derive(Component, Reflect)]
	pub struct Foobar;

	#[test]
	fn works() -> Result<()> {
		let _node = BeetNode::new(EmptyAction);
		let _node2 = BeetNode::new((
			EmptyAction,
			Foobar,
			SetOnStart::<Score>::default(),
		));
		let node = EmptyAction.child(
			(EmptyAction, SetOnStart::<Score>::default()).child(EmptyAction),
		);

		let _val = node.into_graph::<EcsNode>();

		Ok(())
	}

	#[test]
	fn into() -> Result<()> {
		fn foo<M>(_val: impl IntoBeetNode<M>) {}

		let _ = foo(EmptyAction.child(EmptyAction));
		let _ = foo(EmptyAction
			.child((EmptyAction, EmptyAction))
			.child(EmptyAction)
			.child(
				(EmptyAction, EmptyAction)
					.child(EmptyAction)
					.child(EmptyAction),
			));


		Ok(())
	}

	#[test]
	fn spawns() -> Result<()> {
		let mut world = World::new();

		let agent = world.spawn_empty().id();

		let root = (Score::default(), SetOnStart(Score::Weight(0.5)))
			.into_beet_node()
			.with_type::<Score>() // not needed by happenstance but usually required
			.spawn(&mut world, agent)
			.value;

		expect(&world).to_have_entity(root)?;
		expect(&world).component::<AgentMarker>(agent)?;
		expect(&world).component(root)?.to_be(&TargetAgent(agent))?;
		expect(&world)
			.component(root)?
			.to_be(&SetOnStart(Score::Weight(0.5)))?;

		// test shared component
		expect(&world).component(root)?.to_be(&Score::default())?;

		Ok(())
	}

	#[test]
	fn default_components() -> Result<()> {
		let mut app = App::new();
		let target = app.world.spawn_empty().id();
		let actions = test_constant_behavior_tree();
		let root = actions.spawn(&mut app.world, target).value;

		expect(&app).to_have_component::<SetOnStart<Score>>(root)?;
		expect(&app).to_have_component::<TargetAgent>(root)?;
		expect(&app).to_have_component::<RunTimer>(root)?;
		expect(&app).to_have_component::<Score>(root)?;

		Ok(())
	}
}
