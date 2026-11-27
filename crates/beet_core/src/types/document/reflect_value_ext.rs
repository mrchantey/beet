//! Shared reflect helpers for the [`ReflectFieldRef`] and [`ResourceFieldRef`]
//! binding syncs: the reflect-serde bridges and the field path check.
//!
//! The bound source is always a component under the hood (resources are
//! entity-backed), so both syncs funnel through the same pair of helpers: a
//! `Value` on `value_entity`, a component on `target`, and a [`GetPath`] access
//! string selecting the field.

use crate::prelude::*;
use bevy::ecs::change_detection::Tick;
use bevy::ecs::component::ComponentId;
use bevy::ecs::reflect::ReflectComponent;
use bevy::reflect::GetPath;
use core::any::TypeId;

/// Walk a dot-separated field path through `info`, erroring at the first segment
/// its type does not declare.
///
/// An empty path binds the whole value, so it always passes. The walk
/// stops accepting once a segment's type is not a struct (a collection index, an
/// opaque type): what it can check, it checks.
pub(crate) fn validate_field_path(
	info: &'static bevy::reflect::TypeInfo,
	field: &str,
) -> Result<()> {
	use bevy::reflect::TypeInfo;
	let mut info = info;
	for segment in field.split('.').filter(|segment| !segment.is_empty()) {
		let TypeInfo::Struct(struct_info) = info else {
			return Ok(());
		};
		let Some(named) = struct_info.field(segment) else {
			bevybail!(
				"`{}` declares no field `{segment}`, so the binding `{field}` would sync nothing",
				struct_info.type_path_table().short_path()
			);
		};
		match named.type_info() {
			Some(next) => info = next,
			None => return Ok(()),
		}
	}
	Ok(())
}

/// Tracks the previous sync pass tick and caches resolved [`ComponentId`]s
/// keyed by the name a binding wrote: the shared shape of the binding sync cache
/// resources ([`ReflectFieldRef`] and [`ResourceFieldRef`] each keep their own,
/// the syncs run at different points so the `last_run` snapshots differ).
#[derive(Default)]
pub(crate) struct BindingTickCache {
	/// The `world.change_tick()` snapshot of the previous sync pass.
	pub last_run: Tick,
	/// Resolved [`ComponentId`] per name.
	pub ids: HashMap<ReflectName, ComponentId>,
}

impl BindingTickCache {
	/// Get or resolve the [`ComponentId`] for `name`, caching on success.
	/// `None` until the type has been registered with the world (eg inserted at
	/// least once), retried next pass.
	pub fn component_id(
		&mut self,
		world: &World,
		name: &ReflectName,
		type_id: TypeId,
	) -> Option<ComponentId> {
		match self.ids.get(name) {
			Some(component_id) => Some(*component_id),
			None => {
				let component_id = world.components().get_id(type_id)?;
				self.ids.insert(name.clone(), component_id);
				Some(component_id)
			}
		}
	}
}

/// The [`GetPath`] access string for a bound field, ie `.field.path` (empty for
/// the whole value).
pub fn field_access(field: &str) -> String {
	if field.is_empty() {
		String::new()
	} else {
		format!(".{field}")
	}
}

/// Reflect-write the field at `access` of `target`'s component from
/// `value_entity`'s [`Value`].
pub fn write_field_from_value(
	world: &mut World,
	value_entity: Entity,
	target: Entity,
	access: &str,
	reflect_component: &ReflectComponent,
) {
	let Some(value) = world.entity(value_entity).get::<Value>().cloned() else {
		return;
	};
	let registry = world.resource::<AppTypeRegistry>().clone();
	let registry = registry.read();
	// the bound field's current type, so the Value resolves to the concrete type.
	let field_type_id = reflect_component
		.reflect(world.entity(target))
		.and_then(|component| component.reflect_path(access).ok())
		.and_then(|field| field.get_represented_type_info())
		.map(|info| info.type_id());
	let patch = match value_to_reflect(&value, field_type_id, &registry) {
		Ok(Some(patch)) => patch,
		Ok(None) => return,
		// a malformed authored value (a unit-less duration typed into a bound
		// `Duration` field) is the human's, so it is raised rather than dropped.
		Err(err) => return world.handle_command_error::<Value>(err),
	};
	if let Some(mut component) =
		reflect_component.reflect_mut(world.entity_mut(target))
		&& let Ok(field_mut) = component.reflect_path_mut(access)
	{
		field_mut.apply(patch.as_ref());
	}
}

/// Reflect-read the field at `access` of `target`'s component into
/// `value_entity`'s [`Value`], guarded on inequality so it never dirties
/// [`Value`] (and thus the document) unless it differs.
pub fn read_field_into_value(
	world: &mut World,
	value_entity: Entity,
	target: Entity,
	access: &str,
	reflect_component: &ReflectComponent,
) {
	let registry = world.resource::<AppTypeRegistry>().clone();
	let registry = registry.read();
	let read = reflect_component
		.reflect(world.entity(target))
		.and_then(|component| component.reflect_path(access).ok())
		.and_then(|field| reflect_to_value(field, &registry));
	let Some(new_value) = read else {
		return;
	};
	if let Some(mut value) = world.entity_mut(value_entity).get_mut::<Value>() {
		value.set_if_neq(new_value);
	}
}

/// Serialize a reflected value to a [`Value`] via reflect serde.
pub fn reflect_to_value(
	value: &dyn bevy::reflect::PartialReflect,
	registry: &bevy::reflect::TypeRegistry,
) -> Option<Value> {
	let serializer =
		bevy::reflect::serde::TypedReflectSerializer::new(value, registry);
	Value::from_serde(serializer).ok()
}

/// A reflect patch as the [`Value`] its type's serde form writes, only the
/// fields the patch holds: what a document records of a component whose other
/// fields fill from its default.
///
/// Reflect serde writes a partial struct's fields by position, so a patch is
/// walked here and only its concrete values reach the serializer. A type with
/// a serde of its own is whole or nothing, so a partial struct of one is
/// refused, its other fields the default's to mint and no document's.
#[cfg(feature = "serde")]
pub fn patch_to_value(
	patch: &dyn bevy::reflect::PartialReflect,
	registry: &bevy::reflect::TypeRegistry,
) -> Result<Value> {
	use bevy::reflect::ReflectDeserialize;
	use bevy::reflect::ReflectFromReflect;
	use bevy::reflect::ReflectRef;
	use bevy::reflect::ReflectSerialize;
	use bevy::reflect::TypeInfo;
	use bevy::reflect::enums::VariantType;

	let concrete = |value: &dyn bevy::reflect::PartialReflect| {
		Value::from_serde(bevy::reflect::serde::TypedReflectSerializer::new(
			value, registry,
		))
	};
	if !patch.is_dynamic() {
		return concrete(patch);
	}
	let info = patch.get_represented_type_info();
	if let Some(registration) =
		info.and_then(|info| registry.get(info.type_id()))
		&& (registration.data::<ReflectSerialize>().is_some()
			|| registration.data::<ReflectDeserialize>().is_some())
	{
		// its own serde form has no partial: completing one here would write
		// down a default minted for each value, ie an id
		if let (ReflectRef::Struct(patch), TypeInfo::Struct(info)) =
			(patch.reflect_ref(), registration.type_info())
			&& patch.field_len() < info.field_len()
		{
			bevybail!(
				"`{}` has no partial serde form",
				registration.type_info().type_path()
			);
		}
		return registration
			.data::<ReflectFromReflect>()
			.and_then(|from_reflect| from_reflect.from_reflect(patch))
			.ok_or_else(|| {
				bevyhow!(
					"`{}` cannot be completed from its fields",
					registration.type_info().type_path()
				)
			})
			.and_then(|value| concrete(value.as_partial_reflect()));
	}
	let each = |value: &dyn bevy::reflect::PartialReflect| {
		patch_to_value(value, registry)
	};
	match patch.reflect_ref() {
		ReflectRef::Struct(value) => (0..value.field_len())
			.filter_map(|index| value.name_at(index).zip(value.field_at(index)))
			.map(|(name, field)| (SmolStr::new(name), each(field)?).xok())
			.collect::<Result<Map>>()?
			.xmap(Value::Map),
		// a newtype is its field
		ReflectRef::TupleStruct(value)
			if value.field_len() == 1
				&& matches!(
					info,
					Some(TypeInfo::TupleStruct(info)) if info.field_len() == 1
				) =>
		{
			each(value.field(0).expect("one field"))?
		}
		ReflectRef::TupleStruct(value) => value
			.iter_fields()
			.map(each)
			.collect::<Result<Vec<_>>>()?
			.xmap(Value::List),
		ReflectRef::Tuple(value) => value
			.iter_fields()
			.map(each)
			.collect::<Result<Vec<_>>>()?
			.xmap(Value::List),
		ReflectRef::List(value) => value
			.iter()
			.map(each)
			.collect::<Result<Vec<_>>>()?
			.xmap(Value::List),
		ReflectRef::Array(value) => value
			.iter()
			.map(each)
			.collect::<Result<Vec<_>>>()?
			.xmap(Value::List),
		ReflectRef::Enum(value) => {
			let fields = value.iter_fields().collect::<Vec<_>>();
			// an option is its value or null, as serde writes one
			if info.is_some_and(|info| {
				reflect_ext::option_some_inner(info).is_some()
			}) {
				return match fields.first() {
					Some(field) => each(field.value()),
					None => Value::Null.xok(),
				};
			}
			let fields = match value.variant_type() {
				VariantType::Unit => {
					return Value::str(value.variant_name()).xok();
				}
				VariantType::Struct => fields
					.iter()
					.map(|field| {
						(
							SmolStr::new(field.name().unwrap_or_default()),
							each(field.value())?,
						)
							.xok()
					})
					.collect::<Result<Map>>()?
					.xmap(Value::Map),
				VariantType::Tuple => match fields.as_slice() {
					[field] => each(field.value())?,
					fields => fields
						.iter()
						.map(|field| each(field.value()))
						.collect::<Result<Vec<_>>>()?
						.xmap(Value::List),
				},
			};
			Value::Map(Map::new([(SmolStr::new(value.variant_name()), fields)]))
		}
		_ => bevybail!(
			"`{}` has no partial serde form",
			patch.reflect_type_path()
		),
	}
	.xok()
}

/// Deserialize a [`Value`] into a reflected value of `type_id`, taking the
/// type's authored spelling first and its reflect serde otherwise.
///
/// The write-back end of the seam contract: a [`LiteralParser`] that accepts
/// the value wins, so a human typing `"30s"` into a bound `Duration` field
/// means that duration (serde refuses it); one that declines, or a type with no
/// entry, continues into the deserializer, so a structural [`Value::Map`] never
/// shadows serde. The read direction ([`reflect_to_value`]) stays serde.
///
/// Serde reads the [`Value`] directly through a [`ValueDeserializer`], and a
/// value it cannot read is `Ok(None)` rather than an error, as a binding syncs
/// every pass.
pub fn value_to_reflect(
	value: &Value,
	type_id: Option<TypeId>,
	registry: &bevy::reflect::TypeRegistry,
) -> Result<Option<Box<dyn bevy::reflect::PartialReflect>>> {
	use serde::de::DeserializeSeed;
	let Some(type_id) = type_id else {
		return Ok(None);
	};
	if let Some(reflected) = LiteralParser::parse_type(type_id, value)? {
		return Ok(Some(reflected));
	}
	// serde needs the type registered; the parser above does not.
	let Some(registration) = registry.get(type_id) else {
		return Ok(None);
	};
	bevy::reflect::serde::TypedReflectDeserializer::new(registration, registry)
		.deserialize(ValueDeserializer::new(value.clone()))
		.ok()
		.xok()
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::FromReflect;
	use bevy::reflect::TypeRegistry;
	use core::time::Duration;

	/// A human typing an authored spelling into a bound field means what they
	/// wrote: serde refuses `"30s"` for a `Duration`, whose structural form is
	/// a seconds/nanos pair.
	#[crate::test]
	fn writes_back_an_authored_spelling() {
		let registry = TypeRegistry::default();
		value_to_reflect(
			&Value::str("30s"),
			Some(TypeId::of::<Duration>()),
			&registry,
		)
		.unwrap()
		.map(|reflected| Duration::from_reflect(reflected.as_ref()).unwrap())
		.xpect_eq(Some(Duration::from_secs(30)));
		// a malformed one is reported, never silently dropped
		value_to_reflect(
			&Value::str("30"),
			Some(TypeId::of::<Duration>()),
			&registry,
		)
		.xpect_err();
	}

	/// A parser that declines the shape continues into serde, so a structural
	/// value never loses to a partial parser.
	#[crate::test]
	fn a_declined_shape_falls_through_to_serde() {
		value_to_reflect(
			&Value::Bool(true),
			Some(TypeId::of::<bool>()),
			&TypeRegistry::default(),
		)
		.unwrap()
		.map(|reflected| bool::from_reflect(reflected.as_ref()).unwrap())
		.xpect_eq(Some(true));
	}

	/// A byte string writes back into a byte list, and an unsigned integer
	/// into a signed field, as serde reads the [`Value`] directly.
	#[crate::test]
	fn writes_back_bytes_and_integers() {
		let mut registry = TypeRegistry::default();
		registry.register::<Vec<u8>>();
		value_to_reflect(
			&Value::Bytes(vec![0, 255]),
			Some(TypeId::of::<Vec<u8>>()),
			&registry,
		)
		.unwrap()
		.map(|reflected| Vec::<u8>::from_reflect(reflected.as_ref()).unwrap())
		.xpect_eq(Some(vec![0, 255]));
		value_to_reflect(&Value::Uint(7), Some(TypeId::of::<i32>()), &registry)
			.unwrap()
			.map(|reflected| i32::from_reflect(reflected.as_ref()).unwrap())
			.xpect_eq(Some(7));
	}
}
