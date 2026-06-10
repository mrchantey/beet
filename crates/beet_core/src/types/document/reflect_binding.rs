//! Reflect-field binding: a [`FieldRef`] targeting a field of an arbitrary
//! component, bidirectionally, the generalization of the `Value`-component bind.
//!
//! A `<MyComponent value=#path>` writes the document value into `MyComponent.value`
//! and reads it back. The [`FieldRef`]'s synced [`Value`] is the intermediary: the
//! document sync keeps `Value` in step with the document, and this layer keeps
//! `Value` in step with a reflected component field, so the full chain is
//! `document <-> Value <-> MyComponent.field`.
//!
//! Both directions gate on inequality:
//!
//! - read (`Value` -> component): when the [`Value`] changes (the document drove
//!   it, or a local edit), reflect-write the field.
//! - write-back (component -> `Value`): otherwise, reflect-read the field; the
//!   equality guard ([`set_if_neq`](core::cmp)) only dirties [`Value`] when it
//!   differs, so a whole-component change tick is a safe, sufficient signal and no
//!   per-field change detection is needed.
//!
//! Gated behind `json`: the bridge between a [`Value`] and a reflected field goes
//! through serde, and the no_std core stays free of it.

use crate::prelude::*;
use bevy::ecs::reflect::ReflectComponent;
use bevy::reflect::GetPath;
use core::any::TypeId;

/// Binds the co-located [`FieldRef`]'s [`Value`] to a field of a component on the
/// same entity, by the component's short type path and a field path within it.
///
/// Co-locate with a [`FieldRef`] (which supplies the [`Value`] and the document
/// link): `(MyComponent::default(), FieldRef::new("path"), ReflectFieldRef::new("MyComponent", "value"))`
/// binds document field `path` to `MyComponent.value`, both ways.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReflectFieldRef {
	/// The target component's short type path, eg `MyComponent`.
	pub component: SmolStr,
	/// The field path within the component, eg `value` or `style.width`. Empty
	/// binds the whole component value.
	pub field: SmolStr,
}

impl ReflectFieldRef {
	/// Bind to `component`'s `field`.
	pub fn new(component: impl Into<SmolStr>, field: impl Into<SmolStr>) -> Self {
		Self {
			component: component.into(),
			field: field.into(),
		}
	}

	/// The `GetPath` access string for the bound field, ie `.field.path` (empty
	/// for the whole component).
	fn access(&self) -> String {
		if self.field.is_empty() {
			String::new()
		} else {
			format!(".{}", self.field)
		}
	}
}

/// Bidirectionally sync every [`ReflectFieldRef`]'s [`Value`] with its bound
/// component field.
///
/// An exclusive system: it reflects into arbitrary component types via the
/// [`AppTypeRegistry`], which a non-exclusive system cannot do over a dynamic
/// type. Runs in the document sync chain after the document drives [`Value`].
pub fn sync_reflect_field_bindings(world: &mut World) {
	// collect the bindings and whether each entity's Value changed this pass.
	let bindings = world
		.query::<(Entity, Ref<Value>, &ReflectFieldRef)>()
		.iter(world)
		.map(|(entity, value, binding)| {
			(entity, value.is_changed(), binding.clone())
		})
		.collect::<Vec<_>>();
	if bindings.is_empty() {
		return;
	}
	let registry = world.resource::<AppTypeRegistry>().clone();
	let registry = registry.read();

	for (entity, value_changed, binding) in bindings {
		let Some(reflect_component) =
			resolve_component(&registry, &binding.component)
		else {
			continue;
		};
		if value_changed {
			// read direction: the Value (document-driven or edited) writes the field.
			write_field_from_value(world, entity, &binding, reflect_component);
		} else {
			// write-back: the component field writes the Value, guarded on inequality.
			read_field_into_value(world, entity, &binding, reflect_component);
		}
	}
}

/// Resolve a component short type path to its [`ReflectComponent`].
fn resolve_component<'a>(
	registry: &'a bevy::reflect::TypeRegistry,
	component: &str,
) -> Option<&'a ReflectComponent> {
	registry
		.get_with_short_type_path(component)?
		.data::<ReflectComponent>()
}

/// Reflect-write the bound field from the entity's [`Value`].
fn write_field_from_value(
	world: &mut World,
	entity: Entity,
	binding: &ReflectFieldRef,
	reflect_component: &ReflectComponent,
) {
	let Some(value) = world.entity(entity).get::<Value>().cloned() else {
		return;
	};
	let registry = world.resource::<AppTypeRegistry>().clone();
	let registry = registry.read();
	let access = binding.access();
	let Some(mut component) =
		reflect_component.reflect_mut(world.entity_mut(entity))
	else {
		return;
	};
	// the bound field's current type, so the Value resolves to the concrete type.
	let Ok(field) = component.reflect_path(access.as_str()) else {
		return;
	};
	let field_type_id = field.get_represented_type_info().map(|info| info.type_id());
	let Some(patch) = value_to_reflect(&value, field_type_id, &registry) else {
		return;
	};
	if let Ok(field_mut) = component.reflect_path_mut(access.as_str()) {
		field_mut.apply(patch.as_ref());
	}
}

/// Reflect-read the bound field into the entity's [`Value`], guarded on inequality
/// so it never dirties [`Value`] (and thus the document) unless it differs.
fn read_field_into_value(
	world: &mut World,
	entity: Entity,
	binding: &ReflectFieldRef,
	reflect_component: &ReflectComponent,
) {
	let registry = world.resource::<AppTypeRegistry>().clone();
	let registry = registry.read();
	let access = binding.access();
	let read = reflect_component
		.reflect(world.entity(entity))
		.and_then(|component| component.reflect_path(access.as_str()).ok())
		.and_then(|field| reflect_to_value(field, &registry));
	let Some(new_value) = read else {
		return;
	};
	if let Some(mut value) = world.entity_mut(entity).get_mut::<Value>() {
		value.set_if_neq(new_value);
	}
}

/// Serialize a reflected value to a [`Value`] via reflect serde.
fn reflect_to_value(
	value: &dyn bevy::reflect::PartialReflect,
	registry: &bevy::reflect::TypeRegistry,
) -> Option<Value> {
	let serializer =
		bevy::reflect::serde::TypedReflectSerializer::new(value, registry);
	Value::from_serde(serializer).ok()
}

/// Deserialize a [`Value`] into a reflected value of `type_id` via reflect serde,
/// bridging through `serde_json::Value` so no private deserializer is needed.
fn value_to_reflect(
	value: &Value,
	type_id: Option<TypeId>,
	registry: &bevy::reflect::TypeRegistry,
) -> Option<Box<dyn bevy::reflect::PartialReflect>> {
	use serde::de::DeserializeSeed;
	let registration = registry.get(type_id?)?;
	let json = serde_json::to_value(value).ok()?;
	let deserializer =
		bevy::reflect::serde::TypedReflectDeserializer::new(registration, registry);
	deserializer.deserialize(json).ok()
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Component, Reflect, Default, Clone, PartialEq, Debug)]
	#[reflect(Component, Default)]
	struct Slider {
		value: i64,
	}

	/// A world with the document plugin plus a registered `Slider`.
	fn world() -> World {
		let mut world = DocumentPlugin::world();
		world.resource_mut::<AppTypeRegistry>().write().register::<Slider>();
		world
	}

	#[beet_core::test]
	fn document_writes_component_field() {
		let mut world = world();
		let doc = world.spawn(Document::new(val!({ "level": 7i64 }))).id();
		// bind document `level` -> Slider.value via the co-located FieldRef Value.
		world.spawn((
			ChildOf(doc),
			Slider::default(),
			Value::default(),
			FieldRef::new("level"),
			ReflectFieldRef::new("Slider", "value"),
		));
		world.update_local();

		let slider = world.query_once::<&Slider>()[0].clone();
		slider.value.xpect_eq(7);
	}

	#[beet_core::test]
	fn component_field_writes_back_to_document() {
		let mut world = world();
		let doc = world.spawn(Document::new(val!({ "level": 0i64 }))).id();
		let entity = world
			.spawn((
				ChildOf(doc),
				Slider::default(),
				Value::default(),
				FieldRef::new("level"),
				ReflectFieldRef::new("Slider", "value"),
			))
			.id();
		// settle the initial document -> component sync.
		world.update_local();
		world.update_local();

		// edit the component directly; the write-back reaches the document.
		world.entity_mut(entity).get_mut::<Slider>().unwrap().value = 42;
		world.update_local();
		world.update_local();

		world
			.entity(doc)
			.get::<Document>()
			.unwrap()
			.get_field::<i64>(&[FieldSegment::key("level")])
			.unwrap()
			.xpect_eq(42);
	}
}
