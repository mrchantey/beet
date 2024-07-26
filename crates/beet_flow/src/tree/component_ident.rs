use crate::prelude::*;
use anyhow::Result;
use bevy::ecs::component::ComponentInfo;
use bevy::prelude::*;
use bevy::reflect::TypeData;
use std::any::TypeId;




#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ComponentIdent {
	pub entity: Entity,
	pub type_id: TypeId,
}


impl ComponentIdent {
	pub fn new(entity: Entity, type_id: TypeId) -> Self {
		Self { entity, type_id }
	}

	pub fn into_field(self) -> FieldIdent { FieldIdent::new(self, Vec::new()) }

	pub fn add(self, world: &mut World) -> Result<()> {
		ComponentUtils::add(world, self.entity, self.type_id)
	}

	pub fn remove(self, world: &mut World) -> Result<()> {
		ComponentUtils::remove(world, self.entity, self.type_id)
	}

	pub fn info(self, world: &World) -> Result<&ComponentInfo> {
		let component_id = ComponentUtils::component_id(world, self.type_id)?;
		world.components().get_info(component_id).ok_or_else(|| {
			anyhow::anyhow!("no info for component id: {:?}", component_id)
		})
	}

	pub fn map<O>(
		self,
		world: &World,
		func: impl FnOnce(&dyn Reflect) -> O,
	) -> Result<O> {
		ComponentUtils::map(world, self.entity, self.type_id, func)
	}

	pub fn map_mut<O>(
		self,
		world: &mut World,
		func: impl FnOnce(&mut dyn Reflect) -> O,
	) -> Result<O> {
		ComponentUtils::map_mut(world, self.entity, self.type_id, func)
	}

	pub fn map_type_data<T: TypeData, O>(
		self,
		world: &World,
		func: impl FnOnce(&T) -> O,
	) -> Result<O> {
		let registry = world.resource::<AppTypeRegistry>().clone();
		let registry = registry.read();
		let Some(reflect) = registry.get_type_data::<T>(self.type_id) else {
			anyhow::bail!("{:?} is not ActionMeta", self.info(world)?);
		};

		Ok(func(reflect))
	}

	pub fn map_action_meta<O>(
		self,
		world: &World,
		func: impl FnOnce(&dyn ActionMeta) -> O,
	) -> Result<O> {
		let out = self
			.map_type_data::<ReflectActionMeta, _>(world, move |reflect| {
				self.map(world, move |c| {
					let meta = reflect
						.get(&*c)
						.ok_or_else(|| anyhow::anyhow!("mismatch"))?;
					Ok(func(meta))
				})
			})
			.flatten()
			.flatten()?;
		Ok(out)
	}
	pub fn category(self, world: &World) -> Result<ActionCategory> {
		self.map_action_meta(world, |meta| meta.category())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use std::any::TypeId;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		let mut registry = world.resource::<AppTypeRegistry>().write();
		registry.register::<EmptyAction>();
		let type_id = TypeId::of::<EmptyAction>();
		drop(registry);

		let entity = world.spawn(EmptyAction).id();
		let component = ComponentIdent::new(entity, type_id);

		expect(component.category(&world)?).to_be(ActionCategory::Behavior)?;


		Ok(())
	}
}
