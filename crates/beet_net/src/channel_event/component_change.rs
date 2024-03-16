use anyhow::Result;
use bevy::prelude::*;
use bevy::reflect::serde::ReflectSerializer;
use bevy::reflect::serde::UntypedReflectDeserializer;
use bevy::reflect::TypeRegistry;
use bincode::Options;
use flume::Receiver;
use flume::Sender;
use std::any::TypeId;

#[derive(Clone)]
pub struct MpscChannel<T> {
	pub send: Sender<T>,
	pub recv: Receiver<T>,
}
impl<T> Default for MpscChannel<T> {
	fn default() -> Self {
		let (send, recv) = flume::unbounded();
		Self { send, recv }
	}
}

#[derive(Clone)]
pub struct ComponentChanged {
	pub id: EntityComponent,
	pub value: Vec<u8>,
}

impl ComponentChanged {
	pub fn new(id: EntityComponent, value: Vec<u8>) -> Self {
		Self { id, value }
	}



	pub fn serialize(
		registry: &TypeRegistry,
		entity: Entity,
		value: &dyn Reflect,
		id: TypeId,
	) -> Result<Self> {
		let serializer = ReflectSerializer::new(value, &registry);
		let bytes = bincode::serialize(&serializer)?;
		let id = EntityComponent::with_id(entity, id);
		Ok(Self::new(id, bytes))
	}

	pub fn serialize_typed<T: Reflect>(
		registry: &TypeRegistry,
		entity: Entity,
		value: &T,
	) -> Result<Self> {
		Self::serialize(registry, entity, value, TypeId::of::<T>())
	}

	pub fn deserialize(
		&self,
		type_registry: &TypeRegistry,
	) -> Result<Box<dyn Reflect>> {
		let new_value = bincode::DefaultOptions::new()
			.with_fixint_encoding()
			.deserialize_seed(
			UntypedReflectDeserializer::new(&type_registry),
			&self.value,
		)?;
		Ok(new_value)
	}
	pub fn deserialize_typed<T: FromReflect>(
		&self,
		type_registry: &TypeRegistry,
	) -> Result<T> {
		let value = self.deserialize(type_registry)?;
		let value = T::from_reflect(&*value)
			.ok_or_else(|| anyhow::anyhow!("Failed to convert from Reflect"))?;
		Ok(value)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityComponent {
	pub entity: Entity,
	pub type_id: TypeId,
}
impl EntityComponent {
	pub fn new<T: 'static>(entity: Entity) -> Self {
		Self {
			entity,
			type_id: TypeId::of::<T>(),
		}
	}
	pub fn with_id(entity: Entity, type_id: TypeId) -> Self {
		Self { entity, type_id }
	}
}
