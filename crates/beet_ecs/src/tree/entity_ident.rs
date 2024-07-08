use crate::prelude::*;
use anyhow::Result;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use std::any::TypeId;


#[derive(
	Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deref, DerefMut,
)]
pub struct EntityIdent(pub Entity);

impl EntityIdent {
	pub fn new(entity: Entity) -> Self { Self(entity) }

	pub fn deep_clone(self, world: &mut World) -> Result<Self> {
		let entities = ChildrenExt::collect_world(*self, world);
		let scene = DynamicSceneBuilder::from_world(world)
			.extract_entities(entities.into_iter())
			.build();
		self.clone_inner(world, scene)
	}
	/// `src_world` will not be mutated, but querystate doesnt have non-mutable option
	pub fn deep_clone_to_dest(
		self,
		src_world: &mut World,
		dst_world: &mut World,
	) -> Result<Self> {
		let entities = ChildrenExt::collect_world(*self, src_world);
		let scene = DynamicSceneBuilder::from_world(src_world)
			.extract_entities(entities.into_iter())
			.build();
		self.clone_inner(dst_world, scene)
	}

	fn clone_inner(
		self,
		world: &mut World,
		scene: DynamicScene,
	) -> Result<Self> {
		let mut entity_map = EntityHashMap::default();
		scene.write_to_world(world, &mut entity_map)?;
		let new_root = entity_map[&*self];

		Ok(Self::new(new_root))
	}

	pub fn children(self, world: &World) -> Vec<EntityIdent> {
		world
			.get::<Children>(*self)
			.map(|children| {
				children.iter().map(|entity| EntityIdent(*entity)).collect()
			})
			.unwrap_or_default()
	}
	pub fn add_default_component(
		self,
		world: &mut World,
		component: TypeId,
	) -> Result<()> {
		ComponentIdent::new(*self, component).add(world)
	}
	pub fn add_default_bundle(
		self,
		world: &mut World,
		bundle: TypeId,
	) -> Result<()> {
		let Some(bundle_id) = world.bundles().get_id(bundle) else {
			anyhow::bail!("Bundle not found: {:?}", bundle);
		};
		let bundle_info = world.bundles().get(bundle_id).expect("just checked");
		for component in bundle_info
			.iter_components()
			.collect::<Vec<_>>()
			.into_iter()
		{
			if let Some(type_id) = world
				.components()
				.get_info(component)
				.expect("just checked")
				.type_id()
			{
				ComponentIdent::new(*self, type_id).add(world)?;
			}
		}
		Ok(())
	}


	pub fn despawn_recursive(self, world: &mut World) {
		despawn_with_children_recursive(world, *self);
	}

	pub fn components(self, world: &World) -> Vec<ComponentIdent> {
		ComponentUtils::get(world, *self)
			.into_iter()
			.map(|c| ComponentIdent::new(*self, c))
			.collect()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;


	fn world() -> World {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		let registry = world.resource_mut::<AppTypeRegistry>();
		let mut registry = registry.write();
		registry.register::<SetOnRun<RunResult>>();
		registry.register::<BeetRoot>();
		drop(registry);
		world
	}

	#[test]
	fn deep_clone() -> Result<()> {
		let mut world = world();
		let entity1 = world
			.spawn((BeetRoot::default(), Running, SetOnRun(RunResult::Success)))
			.id();
		let entity1 = EntityIdent::new(entity1);

		expect(world.entities().len()).to_be(1)?;

		let node2 = entity1.deep_clone(&mut world)?;

		expect(world.entities().len()).to_be(2)?;

		expect(&world)
			.component(*node2)?
			.to_be(&SetOnRun(RunResult::Success))?;

		expect(EntityIdent::get_roots(&mut world).len()).to_be(2)?;

		Ok(())
	}

	#[test]
	fn children() -> Result<()> {
		let mut world = World::new();
		let node = test_no_action_behavior_tree(&mut world).ident();

		expect(node.children(&world).len()).to_be(2)?;
		let child = node.add_child_behavior(&mut world);
		expect(node.children(&world).len()).to_be(3)?;
		child.despawn_recursive(&mut world);
		expect(node.children(&world).len()).to_be(2)?;

		Ok(())
	}
	#[test]
	fn components() -> Result<()> {
		let mut world = World::new();
		let node = test_no_action_behavior_tree(&mut world).ident();

		expect(node.components(&world).len()).to_be_greater_than(0)?;

		Ok(())
	}
}
