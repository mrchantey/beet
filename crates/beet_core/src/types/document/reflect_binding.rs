//! Reflect-field binding: a [`FieldRef`] targeting a field of an arbitrary
//! component, bidirectionally, the generalization of the `Value`-component bind.
//!
//! A `<MyComponent value=@doc:path>` writes the document value into `MyComponent.value`
//! and reads it back. The [`FieldRef`]'s synced [`Value`] is the intermediary: the
//! document sync keeps `Value` in step with the document, and this layer keeps
//! `Value` in step with a reflected component field, so the full chain is
//! `document <-> Value <-> MyComponent.field`.
//!
//! The bound component lives on the [`BindingTarget`]: the binding entity itself
//! by default, or another entity (eg an attribute `Value` binding to its
//! element). A [`BindingTarget::Reserved`] target re-resolves each pass to the
//! nearest self-or-ancestor carrying the named marker component (the lazy
//! reserved refs `@entity:RenderRoot::`/`@entity:Router::`), staying silent until
//! the marker is reachable and forcing a read-back when it first resolves. The
//! `Value` always stays on the binding entity.
//!
//! Both directions are change-detected:
//!
//! - read (`Value` -> component): when the [`Value`] changes (the document drove
//!   it, or a local edit), reflect-write the field.
//! - write-back (component -> `Value`): when the target's component change tick
//!   fired since the last sync pass, reflect-read the field back, guarded on
//!   inequality ([`set_if_neq`](core::cmp)). Like [`ResourceFieldRef`], the
//!   ticks are compared against a manually tracked `last_run` snapshot (the
//!   sync is exclusive, so bevy does not track it for us).
//!
//! Gated behind `json`: the bridge between a [`Value`] and a reflected field goes
//! through serde, and the no_std core stays free of it.

use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::ecs::reflect::ReflectComponent;

/// Binds this entity's [`Value`] to a field of a component on the [`BindingTarget`]
/// entity, by the component's short type path and a field path within it.
///
/// Co-locate with a [`FieldRef`] (which supplies the [`Value`] and the document
/// link): `(MyComponent::default(), FieldRef::new("path"), ReflectFieldRef::new("MyComponent", "value"))`
/// binds document field `path` to `MyComponent.value`, both ways. Use
/// [`with_target`](Self::with_target) to bind a component on another entity.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect, MapEntities)]
#[reflect(Component, MapEntities)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReflectFieldRef {
	/// The target component's short type path, eg `MyComponent`.
	pub component: SmolStr,
	/// The field path within the component, eg `value` or `style.width`. Empty
	/// binds the whole component value.
	pub field: SmolStr,
	/// The entity carrying the bound component.
	#[entities]
	pub target: BindingTarget,
}

impl ReflectFieldRef {
	/// Bind to `component`'s `field` on this entity.
	pub fn new(component: impl Into<SmolStr>, field: impl Into<SmolStr>) -> Self {
		Self {
			component: component.into(),
			field: field.into(),
			target: BindingTarget::This,
		}
	}

	/// Retarget the bound component: an entity, or a lazily resolved
	/// [`BindingTarget::Reserved`] name.
	pub fn with_target(mut self, target: impl Into<BindingTarget>) -> Self {
		self.target = target.into();
		self
	}

	/// The `GetPath` access string for the bound field, ie `.field.path` (empty
	/// for the whole component).
	fn access(&self) -> String { reflect_value_ext::field_access(&self.field) }
}

/// Bidirectionally sync every [`ReflectFieldRef`]'s [`Value`] with its bound
/// component field on the target entity.
///
/// An exclusive system: it reflects into arbitrary component types via the
/// [`AppTypeRegistry`], which a non-exclusive system cannot do over a dynamic
/// type. Runs in the document sync chain after the document drives [`Value`].
/// A missing component or target is silently skipped.
pub fn sync_reflect_field_bindings(world: &mut World) {
	// collect the bindings and each Value's change state this pass.
	let bindings = world
		.query::<(Entity, Ref<Value>, &ReflectFieldRef)>()
		.iter(world)
		.map(|(entity, value, binding)| {
			// a freshly spawned Null Value carries no signal, let the component seed it.
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
	world.init_resource::<ComponentBindingCache>();
	world.resource_scope(|world, mut cache: Mut<ComponentBindingCache>| {
		let last_run = cache.ticks.last_run;
		cache.ticks.last_run = this_run;
		for (entity, value_changed, value_added, binding) in bindings {
			let Some((reflect_component, component_id)) = resolve_component(
				world,
				&registry,
				&mut cache.ticks,
				&binding.component,
			) else {
				continue;
			};
			// silently skipped until a lazy reserved target resolves.
			let Some(target) = binding.target.resolve(world, entity) else {
				continue;
			};
			if world.get_entity(target).is_err() {
				continue;
			}
			// a reserved target resolving for the first time (or to a new
			// entity, eg a reparent) forces a read-back: the component's change
			// tick may have fired long before the marker became reachable.
			let target_resolved = matches!(
				binding.target,
				BindingTarget::Reserved(_)
			) && cache.reserved_targets.insert(entity, target)
				!= Some(target);
			let access = binding.access();
			if value_changed {
				// read direction: the Value (document-driven or edited) writes the field.
				reflect_value_ext::write_field_from_value(
					world,
					entity,
					target,
					&access,
					reflect_component,
				);
			} else if value_added
				|| target_resolved
				|| world
					.entity(target)
					.get_change_ticks_by_id(component_id)
					.is_some_and(|ticks| ticks.is_changed(last_run, this_run))
			{
				// write-back: only when the component change tick fired (or first sync).
				reflect_value_ext::read_field_into_value(
					world,
					entity,
					target,
					&access,
					reflect_component,
				);
			}
		}
	});
}

/// The component-binding sync cache: the shared
/// [`BindingTickCache`](reflect_value_ext::BindingTickCache) plus the last
/// resolved entity per lazy [`BindingTarget::Reserved`] binding, whose change
/// forces a read-back.
#[derive(Default, Resource)]
struct ComponentBindingCache {
	ticks: reflect_value_ext::BindingTickCache,
	reserved_targets: HashMap<Entity, Entity>,
}

/// Resolve a component short type path to its [`ReflectComponent`] and
/// [`ComponentId`], caching the id.
fn resolve_component<'a>(
	world: &World,
	registry: &'a bevy::reflect::TypeRegistry,
	cache: &mut reflect_value_ext::BindingTickCache,
	component: &SmolStr,
) -> Option<(&'a ReflectComponent, ComponentId)> {
	let registration = registry.get_with_short_type_path(component)?;
	let reflect_component = registration.data::<ReflectComponent>()?;
	let component_id =
		cache.component_id(world, component, registration.type_id())?;
	Some((reflect_component, component_id))
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

	#[beet_core::test]
	fn cross_entity_syncs_both_directions() {
		let mut world = world();
		let target = world.spawn(Slider::default()).id();
		// Value on the binding entity, component on the target entity.
		let entity = world
			.spawn((
				Value::Int(7),
				ReflectFieldRef::new("Slider", "value").with_target(target),
			))
			.id();
		world.update_local();

		world.entity(target).get::<Slider>().unwrap().value.xpect_eq(7);

		// edit the target's component; the write-back reaches the binding Value.
		world.entity_mut(target).get_mut::<Slider>().unwrap().value = 42;
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(42));
	}

	#[beet_core::test]
	fn unchanged_component_skips_read_back() {
		let mut world = world();
		let target = world.spawn(Slider { value: 5 }).id();
		let entity = world
			.spawn((
				Value::default(),
				ReflectFieldRef::new("Slider", "value").with_target(target),
			))
			.id();
		// settle the initial component -> Value sync.
		world.update_local();
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(5));

		// diverge the Value without marking it changed: a tick-gated sync leaves
		// it alone, an ungated one would reflect-read the component back over it.
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

		// a real component change resumes the read-back.
		world.entity_mut(target).get_mut::<Slider>().unwrap().value = 6;
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(6));
	}

	#[beet_core::test]
	fn missing_component_is_silent() {
		let mut world = world();
		// `Slider` exists in the world but not on the target.
		world.spawn(Slider::default());
		let target = world.spawn_empty().id();
		let entity = world
			.spawn((
				Value::default(),
				ReflectFieldRef::new("Slider", "value").with_target(target),
			))
			.id();
		// an entirely unknown component type is also silent.
		world.spawn((Value::default(), ReflectFieldRef::new("Nope", "field")));
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Null);
	}

	/// An unreflected marker: the reserved resolution falls back to the
	/// component-info scan by short name.
	#[derive(Component)]
	struct Marker;

	#[beet_core::test]
	fn reserved_target_resolves_nearest_marker_ancestor() {
		let mut world = world();
		let marked = world.spawn((Marker, Slider { value: 3 })).id();
		let mid = world.spawn(ChildOf(marked)).id();
		let entity = world
			.spawn((
				ChildOf(mid),
				Value::default(),
				ReflectFieldRef::new("Slider", "value")
					.with_target(BindingTarget::Reserved("Marker".into())),
			))
			.id();
		world.update_local();

		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(3));

		// reactive both ways: a component edit reaches the Value...
		world.entity_mut(marked).get_mut::<Slider>().unwrap().value = 6;
		world.update_local();
		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(6));

		// ...and a Value edit reaches the marker's component.
		*world.entity_mut(entity).get_mut::<Value>().unwrap() =
			Value::Int(11);
		world.update_local();
		world.entity(marked).get::<Slider>().unwrap().value.xpect_eq(11);
	}

	#[beet_core::test]
	fn reserved_target_silent_until_marker_attaches() {
		let mut world = world();
		// the binding spawns detached: no marker in its ancestry yet.
		let entity = world
			.spawn((
				Value::default(),
				ReflectFieldRef::new("Slider", "value")
					.with_target(BindingTarget::Reserved("Marker".into())),
			))
			.id();
		// register Marker's component id (the world has never seen one).
		world.register_component::<Marker>();
		world.update_local();
		world.update_local();
		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Null);

		// attaching beneath a marker picks the binding up, even though the
		// component's change tick fired before the marker was reachable.
		let marked = world.spawn((Marker, Slider { value: 9 })).id();
		world.update_local();
		world.entity_mut(entity).insert(ChildOf(marked));
		world.update_local();
		world
			.entity(entity)
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::Int(9));
	}

	#[beet_core::test]
	fn target_entity_is_scene_mappable() {
		let mut world = world();
		// the registration carries the entity-mapping type data for scene loads.
		world
			.resource::<AppTypeRegistry>()
			.read()
			.get(core::any::TypeId::of::<ReflectFieldRef>())
			.unwrap()
			.data::<ReflectMapEntities>()
			.is_some()
			.xpect_true();

		// mapping remaps the Entity target variant.
		let old_target = world.spawn_empty().id();
		let new_target = world.spawn_empty().id();
		let mut binding =
			ReflectFieldRef::new("Slider", "value").with_target(old_target);
		let mut mapping =
			<bevy::ecs::entity::EntityHashMap<Entity>>::default();
		mapping.insert(old_target, new_target);
		binding.map_entities(&mut mapping);
		binding.target.xpect_eq(BindingTarget::Entity(new_target));
	}
}
