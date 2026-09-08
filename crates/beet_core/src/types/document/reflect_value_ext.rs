//! Shared reflect-serde bridges between a [`Value`] and a reflected field,
//! used by the [`ReflectFieldRef`] and [`ResourceFieldRef`] binding syncs.
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

/// Tracks the previous sync pass tick and caches resolved [`ComponentId`]s
/// keyed by short type path: the shared shape of the binding sync cache
/// resources ([`ReflectFieldRef`] and [`ResourceFieldRef`] each keep their own,
/// the syncs run at different points so the `last_run` snapshots differ).
#[derive(Default)]
pub(crate) struct BindingTickCache {
	/// The `world.change_tick()` snapshot of the previous sync pass.
	pub last_run: Tick,
	/// Resolved [`ComponentId`] per short type path.
	pub ids: HashMap<SmolStr, ComponentId>,
}

impl BindingTickCache {
	/// Get or resolve the [`ComponentId`] for `type_path`, caching on success.
	/// `None` until the type has been registered with the world (eg inserted at
	/// least once), retried next pass.
	pub fn component_id(
		&mut self,
		world: &World,
		type_path: &SmolStr,
		type_id: TypeId,
	) -> Option<ComponentId> {
		match self.ids.get(type_path) {
			Some(component_id) => Some(*component_id),
			None => {
				let component_id = world.components().get_id(type_id)?;
				self.ids.insert(type_path.clone(), component_id);
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

/// Deserialize a [`Value`] into a reflected value of `type_id`, taking the
/// type's authored spelling first and its reflect serde otherwise.
///
/// The write-back end of the seam contract: a [`LiteralParser`] that accepts
/// the value wins, so a human typing `"30s"` into a bound `Duration` field
/// means that duration (serde refuses it); one that declines, or a type with no
/// entry, continues into the deserializer, so a structural [`Value::Map`] never
/// shadows serde. The read direction ([`reflect_to_value`]) stays serde.
///
/// Serde bridges through `serde_json::Value` so no private deserializer is
/// needed, and a value it cannot read is `Ok(None)` rather than an error, as
/// a binding syncs every pass.
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
	let json = serde_json::to_value(value)?;
	bevy::reflect::serde::TypedReflectDeserializer::new(registration, registry)
		.deserialize(json)
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
}
