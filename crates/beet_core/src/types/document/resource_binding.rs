//! Resource-field binding: binds a co-located [`Value`] to a field of a
//! reflected resource, the resource counterpart of [`ReflectFieldRef`].
//!
//! A `{@res:PackageConfig.title}` keeps the binding entity's [`Value`] in step
//! with `PackageConfig.title`, both ways:
//!
//! - read (`Value` -> resource): when the [`Value`] changes, reflect-write the
//!   resource field.
//! - write-back (resource -> `Value`): when the resource's change tick fired
//!   since the last sync pass, reflect-read the field back, guarded on
//!   inequality ([`set_if_neq`](core::cmp)).
//!
//! Unlike the same-entity component bind, the write-back is gated on real
//! change detection: the resource's [`ComponentId`] ticks are compared against
//! a manually tracked `last_run` snapshot (the sync is exclusive, so bevy does
//! not track it for us), and the reflect-serde read only runs for
//! actually-changed resources.
//!
//! Gated behind `json` like [`ReflectFieldRef`]: the `Value`-field bridge goes
//! through serde.

use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;

/// Binds this entity's [`Value`] to a field of a reflected resource, by the
/// resource's short type path and a field path within it.
///
/// Standalone `(Value::default(), ResourceFieldRef::new("Theme", "contrast"))`
/// mirrors the resource field into the [`Value`]; co-locate a [`FieldRef`] to
/// also link it to a document field, chaining `resource <-> Value <-> document`.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceFieldRef {
	/// The target resource's short type path, eg `PackageConfig`.
	pub resource: SmolStr,
	/// The field path within the resource, eg `title` or `style.width`. Empty
	/// binds the whole resource value.
	pub field: SmolStr,
}

impl ResourceFieldRef {
	/// Bind to `resource`'s `field`.
	pub fn new(
		resource: impl Into<SmolStr>,
		field: impl Into<SmolStr>,
	) -> Self {
		Self {
			resource: resource.into(),
			field: field.into(),
		}
	}

	/// The `GetPath` access string for the bound field, ie `.field.path` (empty
	/// for the whole resource).
	fn access(&self) -> String { reflect_value_ext::field_access(&self.field) }
}

/// Bidirectionally sync every [`ResourceFieldRef`]'s [`Value`] with its bound
/// resource field.
///
/// An exclusive system, like [`sync_reflect_field_bindings`]: it reflects into
/// arbitrary resource types via the [`AppTypeRegistry`]. Runs in the document
/// sync chain after the document drives [`Value`]. A missing or unregistered
/// resource is silently skipped, like a missing component.
pub fn sync_resource_field_bindings(world: &mut World) {
	// collect the bindings and each Value's change state this pass.
	let bindings = world
		.query::<(Entity, Ref<Value>, &ResourceFieldRef)>()
		.iter(world)
		.map(|(entity, value, binding)| {
			// a freshly spawned Null Value carries no signal, let the resource seed it.
			let value_changed =
				value.is_changed() && !(value.is_added() && value.is_null());
			(entity, value_changed, value.is_added(), binding.clone())
		})
		.collect::<Vec<_>>();
	if bindings.is_empty() {
		return;
	}
	let registry = world.resource::<AppTypeRegistry>().clone();
	let registry = registry.read();
	let this_run = world.change_tick();
	world.init_resource::<ResourceBindingCache>();
	world.resource_scope(|world, mut cache: Mut<ResourceBindingCache>| {
		let last_run = cache.last_run;
		cache.last_run = this_run;
		for (entity, value_changed, value_added, binding) in bindings {
			let Some((reflect_component, component_id)) = resolve_resource(
				world,
				&registry,
				cache.as_mut(),
				&binding.resource,
			) else {
				continue;
			};
			let Some(resource_entity) =
				world.resource_entities().get(component_id)
			else {
				continue;
			};
			let access = binding.access();
			if value_changed {
				// read direction: the Value (document-driven or edited) writes the field.
				reflect_value_ext::write_field_from_value(
					world,
					entity,
					resource_entity,
					&access,
					reflect_component,
				);
			} else if value_added
				|| world
					.get_resource_change_ticks_by_id(component_id)
					.is_some_and(|ticks| ticks.is_changed(last_run, this_run))
			{
				// write-back: only when the resource change tick fired (or first sync).
				reflect_value_ext::read_field_into_value(
					world,
					entity,
					resource_entity,
					&access,
					reflect_component,
				);
			}
		}
	});
}

/// The [`BindingTickCache`](reflect_value_ext::BindingTickCache) for resource
/// bindings, so the tick-gated write-back avoids repeated lookups.
#[derive(Default, Resource, Deref, DerefMut)]
struct ResourceBindingCache(reflect_value_ext::BindingTickCache);

/// Resolve a resource short type path to its backing [`ReflectComponent`]
/// (resources are entity-backed components) and [`ComponentId`], caching the id.
fn resolve_resource<'a>(
	world: &World,
	registry: &'a bevy::reflect::TypeRegistry,
	cache: &mut ResourceBindingCache,
	resource: &SmolStr,
) -> Option<(&'a ReflectComponent, ComponentId)> {
	let registration = registry.get_with_short_type_path(resource)?;
	// require ReflectResource so a plain component cannot be bound as a resource.
	registration.data::<ReflectResource>()?;
	let reflect_component = registration.data::<ReflectComponent>()?;
	let component_id =
		cache.component_id(world, resource, registration.type_id())?;
	Some((reflect_component, component_id))
}

#[cfg(test)]
mod test {
	use super::*;

	#[derive(Resource, Reflect, Default, Clone, PartialEq, Debug)]
	#[reflect(Resource, Default)]
	struct Theme {
		contrast: i64,
	}

	/// A world with the document plugin plus a registered `Theme`.
	fn world() -> World {
		let mut world = DocumentPlugin::world();
		world
			.resource_mut::<AppTypeRegistry>()
			.write()
			.register::<Theme>();
		world
	}

	#[beet_core::test]
	fn seeds_value_from_resource() {
		let mut world = world();
		world.insert_resource(Theme { contrast: 5 });
		let entity = world
			.spawn((
				Value::default(),
				ResourceFieldRef::new("Theme", "contrast"),
			))
			.id();
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(5));
	}

	#[beet_core::test]
	fn document_writes_resource_field() {
		let mut world = world();
		world.insert_resource(Theme::default());
		let doc = world.spawn(Document::new(val!({ "level": 7i64 }))).id();
		// bind document `level` -> Theme.contrast via the co-located FieldRef Value.
		world.spawn((
			ChildOf(doc),
			Value::default(),
			FieldRef::new("level"),
			ResourceFieldRef::new("Theme", "contrast"),
		));
		world.update_local();

		world.resource::<Theme>().contrast.xpect_eq(7);
	}

	#[beet_core::test]
	fn resource_field_writes_back_to_document() {
		let mut world = world();
		world.insert_resource(Theme::default());
		let doc = world.spawn(Document::new(val!({ "level": 0i64 }))).id();
		world.spawn((
			ChildOf(doc),
			Value::default(),
			FieldRef::new("level"),
			ResourceFieldRef::new("Theme", "contrast"),
		));
		// settle the initial document -> resource sync.
		world.update_local();
		world.update_local();

		// edit the resource directly; the write-back reaches the document.
		world.resource_mut::<Theme>().contrast = 42;
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

	#[beet_core::test]
	fn unchanged_resource_skips_read_back() {
		let mut world = world();
		world.insert_resource(Theme { contrast: 5 });
		let entity = world
			.spawn((
				Value::default(),
				ResourceFieldRef::new("Theme", "contrast"),
			))
			.id();
		// settle the initial resource -> Value sync.
		world.update_local();
		world.update_local();

		// diverge the Value without marking it changed: a tick-gated sync leaves
		// it alone, an ungated one would reflect-read the resource back over it.
		*world
			.entity_mut(entity)
			.get_mut::<Value>()
			.unwrap()
			.bypass_change_detection() = Value::Int(99);
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(99));

		// a real resource change resumes the read-back.
		world.resource_mut::<Theme>().contrast = 6;
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(6));
	}

	#[beet_core::test]
	fn missing_resource_is_silent() {
		let mut world = world();
		// `Theme` registered but never inserted, plus an entirely unknown type.
		let entity = world
			.spawn((
				Value::default(),
				ResourceFieldRef::new("Theme", "contrast"),
			))
			.id();
		world.spawn((Value::default(), ResourceFieldRef::new("Nope", "field")));
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Null);
	}
}
