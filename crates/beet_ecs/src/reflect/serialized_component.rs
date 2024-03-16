// use anyhow::Result;
// use bevy::ecs::reflect::ReflectMapEntities;
// use bevy::prelude::*;
// use bevy::reflect::serde::UntypedReflectDeserializer;
// use bevy::reflect::TypeData;
// use bevy::reflect::TypeInfo;
// use bevy::reflect::TypeRegistration;
// use bevy::reflect::TypeRegistry;
// use bincode::Options;
// use serde::Deserialize;
// use serde::Serialize;



// /// A way to pass simple components instead of entire scenes.
// /// Using components that reference other entities is not allowed because the references would be invalid in a different world.
// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SerializedComponent {
// 	pub src_entity: Entity,
// 	pub component: Vec<u8>,
// }

 
// impl SerializedComponent {
// 	/// Create a new serialized component.
// 	/// # Errors
// 	/// - If the component is not registered
// 	/// - If the component references entities
// 	pub fn new<T: Component + Reflect>(
// 		src_entity: Entity,
// 		component: T,
// 		type_registry: &TypeRegistry,
// 	) -> Result<Self> {
// 		// entity references are not allowed
// 		let type_info = type_info_generic::<T>(&component)?;
// 		let type_registration = type_registration(&type_registry, &type_info)?;
// 		assert_no_entities(type_registration)?;

// 		// Serialize the component
// 		let reflect_serializer =
// 			ReflectSerializer::new(&component, &type_registry);
// 		let component = bincode::serialize(&reflect_serializer)?;
// 		Ok(SerializedComponent {
// 			src_entity,
// 			component,
// 		})
// 	}

// 	/// This is a combination of reflect deserialize and [`DynamicScene::write_to_world`]
// 	pub fn apply(
// 		&self,
// 		entity: &mut EntityWorldMut,
// 		type_registry: &TypeRegistry,
// 	) -> Result<()> {
// 		let reflect_deserializer =
// 			UntypedReflectDeserializer::new(&type_registry);
// 		let boxed_reflect = bincode::DefaultOptions::new()
// 			.with_fixint_encoding()
// 			.deserialize_seed(reflect_deserializer, &self.component)?;

// 		let type_info = type_info(&boxed_reflect)?;
// 		let type_registration = type_registration(&type_registry, &type_info)?;
// 		let reflect_component =
// 			type_data::<ReflectComponent>(type_registration, type_info)?;


// 		reflect_component.apply_or_insert(
// 			entity,
// 			&*boxed_reflect,
// 			&type_registry,
// 		);

// 		Ok(())
// 	}
// }

// fn type_info_generic<T: Reflect>(
// 	component: &T,
// ) -> Result<&TypeInfo, anyhow::Error> {
// 	component.get_represented_type_info().ok_or_else(|| {
// 		anyhow::anyhow!("Component does not have a represented type.")
// 	})
// }

// fn assert_no_entities(type_registration: &TypeRegistration) -> Result<()> {
// 	if type_registration.data::<ReflectMapEntities>().is_some() {
// 		anyhow::bail!("Components referencing entities cannot be serialized individually.");
// 	} else {
// 		Ok(())
// 	}
// }

// fn type_data<'a, T: TypeData>(
// 	type_registration: &'a TypeRegistration,
// 	type_info: &TypeInfo,
// ) -> std::result::Result<&'a T, SceneSpawnError> {
// 	type_registration.data::<T>().ok_or_else(|| {
// 		SceneSpawnError::UnregisteredComponent {
// 			type_path: type_info.type_path().to_string(),
// 		}
// 	})
// }

// fn type_info(
// 	component: &Box<dyn Reflect>,
// ) -> std::result::Result<&TypeInfo, SceneSpawnError> {
// 	component.get_represented_type_info().ok_or_else(|| {
// 		SceneSpawnError::NoRepresentedType {
// 			type_path: component.reflect_type_path().to_string(),
// 		}
// 	})
// }

// fn type_registration<'a>(
// 	registry: &'a TypeRegistry,
// 	info: &TypeInfo,
// ) -> std::result::Result<&'a TypeRegistration, SceneSpawnError> {
// 	registry.get(info.type_id()).ok_or_else(|| {
// 		SceneSpawnError::UnregisteredButReflectedType {
// 			type_path: info.type_path().to_string(),
// 		}
// 	})
// }
