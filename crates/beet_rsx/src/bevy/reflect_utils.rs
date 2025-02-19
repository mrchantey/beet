use anyhow::Result;
use bevy::ecs::world::FilteredEntityMut;
use bevy::prelude::*;
use bevy::reflect::serde::TypedReflectDeserializer;
use bevy::reflect::DynamicStruct;
use bevy::reflect::TypeInfo;
use bevy::reflect::TypeRegistry;
use serde::de::DeserializeSeed;


/// Various utilities for working with Bevy's reflection system.
pub struct ReflectUtils;


impl ReflectUtils {
	/// For a given field path, split it into the component name and the field path.
	/// If the field path is empty, the second value will be None.
	pub fn split_field_path(field_path: &str) -> (String, Option<String>) {
		let mut parts = field_path.split('.');
		let component_name = parts.next().unwrap();
		let field_path = parts.collect::<Vec<_>>().join(".");
		if field_path.is_empty() {
			(component_name.to_string(), None)
		} else {
			(component_name.to_string(), Some(field_path))
		}
	}

	/// given a key and value, apply the value to the entity's component
	///
	/// ## Parameters
	/// - `field_path`: The fully qualified dot separated path, ie `Transform.translation`
	///
	/// ## TODO
	/// - Nested paths
	pub fn apply_or_insert_at_path(
		registry: &TypeRegistry,
		entity: &mut EntityWorldMut,
		field_path: &str,
		ron_value: &str,
	) -> Result<()> {
		let (component_name, field_path) = Self::split_field_path(field_path);
		let field_path = field_path.as_deref();
		let (reflect_default, reflect_component) =
			Self::reflect_component(&component_name, registry)?;
		if let Some(mut target) = reflect_component.reflect_mut(&mut *entity) {
			Self::apply_reflect(
				registry,
				field_path,
				target.as_reflect_mut(),
				ron_value,
			)?;
		} else {
			let mut default = reflect_default.default();
			Self::apply_reflect(
				registry,
				field_path,
				default.as_mut(),
				ron_value,
			)?;
			reflect_component.insert(
				entity,
				default.as_partial_reflect(),
				registry,
			);
		}
		Ok(())
	}

	/// given a key and value, apply the value to the entity's component
	///
	/// ## Parameters
	/// - `field_path`: The fully qualified dot separated path, ie `Transform.translation`
	///
	/// ## TODO
	/// - Nested paths
	pub fn apply_at_path<'a, T: PartialReflect>(
		registry: &TypeRegistry,
		entity: impl Into<FilteredEntityMut<'a>>,
		field_path: &str,
		value: T,
	) -> Result<()> {
		let (component_name, field_path) = Self::split_field_path(field_path);
		let field_path = field_path.as_deref();
		let (_, reflect_component) =
			Self::reflect_component(&component_name, registry)?;
		let Some(mut target) = reflect_component.reflect_mut(entity) else {
			anyhow::bail!("Could not find component: {}", component_name);
		};
		if let Some(field_path) = field_path {
			let mut dyn_struct = DynamicStruct::default();
			dyn_struct.insert(field_path, value);
			target.apply(&dyn_struct);
		} else {
			todo!("apply direct to component");
		}
		Ok(())
	}

	/// Given a component name, get its reflect component and reflect default
	pub fn reflect_component<'a>(
		key: &str,
		registry: &'a TypeRegistry,
	) -> Result<(&'a ReflectDefault, &'a ReflectComponent)> {
		let registration =
			registry.get_with_short_type_path(key).ok_or_else(|| {
				anyhow::anyhow!(
					"Could not find short type path for key: {}",
					key
				)
			})?;
		let reflect_default =
			registration.data::<ReflectDefault>().ok_or_else(|| {
				anyhow::anyhow!(
					"Could not find reflect default for key: {}",
					key
				)
			})?;
		let reflect_component =
			registration.data::<ReflectComponent>().ok_or_else(|| {
				anyhow::anyhow!(
					"Could not find reflect component for key: {}",
					key
				)
			})?;

		Ok((reflect_default, reflect_component))
	}


	/// With a given target, ie a component, apply the serialized
	/// ron value to the field at the given path.
	/// Path may be empty, in which case the value is applied to the type itself.
	pub fn apply_reflect(
		registry: &TypeRegistry,
		field_path: Option<&str>,
		target: &mut dyn Reflect,
		ron_value: &str,
	) -> Result<()> {
		match target.reflect_type_info() {
			TypeInfo::Struct(info) => {
				if let Some(field_path) = field_path {
					let field = info
						.field(field_path)
						.ok_or_else(|| {
							anyhow::anyhow!(
								"Could not find field {} in struct",
								field_path
							)
						})?
						.type_info()
						.unwrap();
					let registration =
						registry.get(field.type_id()).ok_or_else(|| {
							anyhow::anyhow!(
								"Could not find registration for field type"
							)
						})?;
					let reflect_deserializer =
						TypedReflectDeserializer::new(registration, registry);
					let mut deserializer =
						ron::de::Deserializer::from_str(&ron_value)?;
					let reflect_value =
						reflect_deserializer.deserialize(&mut deserializer)?;
					let mut dyn_struct = DynamicStruct::default();
					dyn_struct.insert_boxed(field_path, reflect_value);
					target.apply(&dyn_struct);
				} else {
					todo!("apply direct to component");
				}
			}
			_ => {
				todo!("other types");
			}
		}
		Ok(())
	}
}
