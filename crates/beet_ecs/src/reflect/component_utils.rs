use anyhow::Result;
use bevy::ecs::component::ComponentId;
use bevy::prelude::*;
use bevy::ptr::OwningPtr;
use bevy::reflect::ReflectFromPtr;
use std::any::TypeId;
use std::ptr::NonNull;

pub struct ComponentUtils;

impl ComponentUtils {
	pub fn get(world: &World, entity: Entity) -> Vec<TypeId> {
		if world.get_entity(entity).is_none() {
			Default::default()
		} else {
			world
				.inspect_entity(entity)
				.into_iter()
				.filter_map(|c| c.type_id())
				.collect()
		}
	}
	pub fn component_id(world: &World, type_id: TypeId) -> Result<ComponentId> {
		world.components().get_id(type_id).ok_or_else(|| {
			anyhow::anyhow!("component not registered: {type_id:?}")
		})
	}


	pub fn set_typed<T: Component>(
		world: &mut World,
		entity: Entity,
		component: T,
	) -> Result<()> {
		world
			.get_entity_mut(entity)
			.map(|mut e| {
				e.insert(component);
			})
			.ok_or_else(|| anyhow::anyhow!("entity not found: {entity:?}"))
	}
	pub fn get_typed<T: Component>(
		world: &mut World,
		entity: Entity,
	) -> Option<&T> {
		world.get_entity(entity).map(|e| e.get::<T>()).flatten()
	}

	pub fn add(
		world: &mut World,
		entity: Entity,
		type_id: TypeId,
	) -> Result<()> {
		let registry = world.resource::<AppTypeRegistry>().clone();
		let registry = registry.read();
		let registration = registry
			.get(type_id)
			.ok_or_else(|| anyhow::anyhow!("type not found: {:?}", type_id))?;
		let reflect_default =
			registration.data::<ReflectDefault>().ok_or_else(|| {
				anyhow::anyhow!("type is not ReflectDefault, try adding #[reflect(Default)]")
			})?;
		let new_value: Box<dyn Reflect> = reflect_default.default();
		let component_id = Self::component_id(world, type_id)?;
		if let Some(mut entity) = world.get_entity_mut(entity) {
			unsafe {
				let non_null =
					NonNull::new_unchecked(Box::into_raw(new_value) as *mut _);
				let ptr = OwningPtr::new(non_null);
				entity.insert_by_id(component_id, ptr);
			}
			Ok(())
		} else {
			anyhow::bail!("entity not found: {entity:?}")
		}
		// let mut node = self.get_node(entity)?;
		// node.components.push(DynComponent::new(new_value.as_ref()));
		// self.set_node(node)?;
	}
	/// Expensive and awkward but we dont have a `world.entity().remove_by_id yet`
	pub fn remove(
		world: &mut World,
		entity: Entity,
		type_id: TypeId,
	) -> Result<()> {
		let Some(component_id) = world.components().get_id(type_id) else {
			anyhow::bail!("component not registered: {type_id:?}")
		};

		let Some(mut entity) = world.get_entity_mut(entity) else {
			anyhow::bail!("entity not found: {entity:?}")
		};
		unsafe { entity.remove_by_id(component_id) };

		Ok(())
	}

	pub fn map<O>(
		world: &World,
		entity: Entity,
		type_id: TypeId,
		func: impl FnOnce(&dyn Reflect) -> O,
	) -> Result<O> {
		let registry = world.resource::<AppTypeRegistry>();
		let registry = registry.read();
		let Some(registration) = registry.get(type_id) else {
			anyhow::bail!("type not registered: {type_id:?}")
		};
		let component_id = Self::component_id(world, type_id)?;
		let Some(entity) = world.get_entity(entity) else {
			anyhow::bail!("entity not found: {entity:?}")
		};
		let Some(component) = entity.get_by_id(component_id) else {
			anyhow::bail!("component not in entity: {type_id:?}")
		};
		let value = unsafe {
			registration
				.data::<ReflectFromPtr>()
				.unwrap()
				.as_reflect(component)
		};

		Ok(func(value))
	}
	pub fn map_mut<O>(
		world: &mut World,
		entity: Entity,
		type_id: TypeId,
		func: impl FnOnce(&mut dyn Reflect) -> O,
	) -> Result<O> {
		let registry = world.resource::<AppTypeRegistry>().clone();
		let registry = registry.read();
		let Some(registration) = registry.get(type_id) else {
			anyhow::bail!("type not registered: {type_id:?}")
		};
		let component_id = Self::component_id(world, type_id)?;
		let Some(mut entity) = world.get_entity_mut(entity) else {
			anyhow::bail!("entity not found: {entity:?}")
		};
		let Some(component) = entity.get_mut_by_id(component_id) else {
			let name = registration.type_info().type_path();
			anyhow::bail!("component not in entity: {name:?}")
		};
		// component.
		let value = unsafe {
			registration
				.data::<ReflectFromPtr>()
				.unwrap()
				.as_reflect_mut(component.into_inner())
		};
		Ok(func(value))
	}
}


#[cfg(test)]
mod test {
	use super::ComponentUtils;
	use anyhow::Result;
	use bevy::prelude::*;
	use std::any::TypeId;
	use sweet::*;

	#[derive(Component, Debug, Default, Reflect, PartialEq)]
	#[reflect(Default)]
	struct MyStruct(pub i32);


	#[test]
	fn get_set() -> Result<()> {
		let mut world = World::new();
		let entity = world.spawn(MyStruct(2)).id();
		let type_id = TypeId::of::<MyStruct>();

		expect(ComponentUtils::get(&world, entity)).to_be(vec![type_id])?;

		let _component_id = ComponentUtils::component_id(&world, type_id)?;

		ComponentUtils::set_typed(&mut world, entity, MyStruct(3))?;
		expect(ComponentUtils::get_typed::<MyStruct>(&mut world, entity))
			.to_be(Some(&MyStruct(3)))?;

		Ok(())
	}
	#[test]
	fn get_set_by_id() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		let registry = world.resource::<AppTypeRegistry>();
		registry.write().register::<MyStruct>();
		world.init_component::<MyStruct>();
		let entity = world.spawn_empty().id();
		let type_id = TypeId::of::<MyStruct>();

		ComponentUtils::add(&mut world, entity, type_id)?;
		expect(ComponentUtils::get(&world, entity)).to_be(vec![type_id])?;

		let type_id2 = ComponentUtils::map(&world, entity, type_id, |e| {
			e.get_represented_type_info().unwrap().type_id()
		})?;
		expect(type_id2).to_be(type_id)?;

		ComponentUtils::remove(&mut world, entity, type_id)?;
		expect(ComponentUtils::get(&world, entity).len()).to_be(0)?;

		Ok(())
	}
}
