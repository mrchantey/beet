//! [`DynamicComponents`]: component types minted at runtime, each holding a
//! [`Value`], with no rust definition behind them.
use crate::prelude::*;
use bevy::ecs::component::ComponentCloneBehavior;
use bevy::ecs::component::ComponentDescriptor;
use bevy::ecs::component::ComponentId;
use bevy::ecs::component::StorageType;
use bevy::ecs::entity::ComponentCloneCtx;
use bevy::ecs::entity::SourceComponent;
use bevy::ptr::OwningPtr;
use bevy::ptr::Ptr;
use core::alloc::Layout;
use core::mem::ManuallyDrop;

/// The runtime component vocabulary this world has minted, mapping each declared
/// name to its live [`ComponentId`] and the [`ValueSchema`] it was declared
/// with.
///
/// Bevy mints a *distinct* component for every descriptor it is handed, even
/// identical ones, so the name is the identity and this map is what keeps a
/// second declaration of the same name from minting a second component.
///
/// A scene carries one by its name, so a saver writes its value under that
/// name and a load inserts it back once the name is minted: a value a body
/// held before its declaration ran (the declaration is usually in the same
/// scene) waits in the entity's [`Unregistered`] and moves in when the name
/// is minted. `beet_action`'s `DynamicComponent` declares one from a scene.
#[derive(Debug, Default, Resource)]
pub struct DynamicComponents {
	by_name: HashMap<SmolStr, Declaration>,
	by_id: HashMap<ComponentId, SmolStr>,
}

/// One minted runtime component: what it is, and what it accepts.
#[derive(Debug)]
struct Declaration {
	id: ComponentId,
	schema: ValueSchema,
}

impl DynamicComponents {
	/// The component registered under `name`, if any.
	pub fn get(world: &World, name: &str) -> Option<ComponentId> {
		world
			.get_resource::<Self>()
			.and_then(|this| this.by_name.get(name))
			.map(|declaration| declaration.id)
	}

	/// The schema `id` was declared with, if it is a runtime component at all.
	pub fn schema_of(world: &World, id: ComponentId) -> Option<&ValueSchema> {
		let this = world.get_resource::<Self>()?;
		this.by_name
			.get(this.by_id.get(&id)?)
			.map(|declaration| &declaration.schema)
	}

	/// Register a runtime component named `name` accepting `schema`, returning
	/// the existing one if the name is already minted with the same schema.
	///
	/// # Errors
	/// Errors when `name` is already minted with a *different* schema. The name
	/// is the identity, so two documents disagreeing about what it means is a
	/// conflict rather than a last-writer-wins: a scene reloading redeclares its
	/// whole vocabulary, and that redeclaration must be a no-op, not a silent
	/// change of meaning.
	pub fn register(
		world: &mut World,
		name: &str,
		schema: ValueSchema,
	) -> Result<ComponentId> {
		if let Some(declaration) =
			world.get_resource::<Self>().and_then(|this| {
				this.by_name.get(name).map(|declaration| {
					(declaration.id, declaration.schema.clone())
				})
			}) {
			let (id, declared) = declaration;
			if declared != schema {
				bevybail!(
					"`{name}` is already declared as {declared:?}, so it cannot \
also be declared as {schema:?}: a runtime component's name is its identity"
				);
			}
			return id.xok();
		}
		// SAFETY: the layout is `Value`, which is `Send + Sync`, so the
		// descriptor's unconditional `is_send_and_sync` holds, and `drop_value`
		// matches it. Every write to this component goes through
		// `Self::insert_value`, which writes exactly one owned `Value`.
		let descriptor = unsafe {
			ComponentDescriptor::new_with_layout(
				name.to_string(),
				StorageType::Table,
				Layout::new::<Value>(),
				Some(drop_value),
				// mutable
				true,
				// no summary tick, the `Component` default
				false,
				ComponentCloneBehavior::Custom(clone_value),
				None,
			)
		};
		let id = world.register_component_with_descriptor(descriptor);
		let mut this = world.get_resource_or_init::<Self>();
		this.by_name.insert(name.into(), Declaration { id, schema });
		this.by_id.insert(id, name.into());
		Self::adopt_held(world, name, id);
		id.xok()
	}

	/// Move every value a load held under `name` into the component just
	/// minted for it, so a scene declaring its vocabulary after the entities
	/// using it loads whole.
	fn adopt_held(world: &mut World, name: &str, id: ComponentId) {
		let holders = world
			.query::<(Entity, &Unregistered)>()
			.iter(world)
			.filter(|(_, held)| held.contains_key(name))
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>();
		for entity in holders {
			let mut entity = world.entity_mut(entity);
			let mut held = entity.take::<Unregistered>().unwrap_or_default();
			if let Some(value) = held.remove(name) {
				Self::insert_value(&mut entity, id, value);
			}
			if !held.is_empty() {
				entity.insert(held);
			}
		}
	}

	/// The value the runtime component `id` holds on `entity`, `None` when it
	/// is absent or `id` is not a runtime component.
	pub fn value<'w>(
		world: &World,
		entity: EntityRef<'w>,
		id: ComponentId,
	) -> Option<&'w Value> {
		Self::name_of(world, id)?;
		// SAFETY: `register` is the only source of a runtime `ComponentId`,
		// checked above, and it always declares a `Value` layout.
		entity
			.get_by_id(id)
			.ok()
			.map(|ptr| unsafe { ptr.deref::<Value>() })
	}

	/// Insert or replace the runtime component `id` on `entity`, holding
	/// `value`. The caller resolves `id` through [`get`](Self::get) or
	/// [`register`](Self::register), the only sources of one.
	pub fn insert_value(
		entity: &mut EntityWorldMut,
		id: ComponentId,
		value: Value,
	) {
		// SAFETY: a runtime `ComponentId` only ever comes from `register`,
		// which declares a `Value` layout and a matching drop fn, and `value`
		// is a `Value` given away here rather than dropped.
		unsafe {
			OwningPtr::make(value, |ptr| {
				entity.insert_by_id(id, ptr);
			})
		};
	}

	/// Insert each value `held` names a minted runtime component, leaving
	/// the rest held: what a load does with a body's unresolved keys.
	pub fn insert_held(entity: &mut EntityWorldMut, held: Unregistered) {
		let mut rest = Unregistered::default();
		for (name, value) in Map::from(EntityBody::clone(&held)) {
			match Self::get(entity.world(), &name) {
				Some(id) => Self::insert_value(entity, id, value),
				None => {
					rest.insert(name, value);
				}
			}
		}
		if !rest.is_empty() {
			entity.insert(rest);
		}
	}

	/// The name `id` was declared under, if it is a runtime component at all.
	pub fn name_of(world: &World, id: ComponentId) -> Option<&SmolStr> {
		world
			.get_resource::<Self>()
			.and_then(|this| this.by_id.get(&id))
	}

	/// Every declared name, for diagnostics.
	///
	/// The descriptor name bevy stores is discarded in a build without its
	/// `debug` feature, so this map is the only reliable id-to-name source.
	pub fn names(world: &World) -> impl Iterator<Item = &SmolStr> {
		world
			.get_resource::<Self>()
			.map(|this| this.by_name.keys())
			.into_iter()
			.flatten()
	}
}

/// Run [`Value`]'s destructor on a runtime component's storage.
///
/// # Safety
/// `ptr` must own a `Value`, which every runtime component's storage does: the
/// descriptor above is the only one that names this fn.
unsafe fn drop_value(ptr: OwningPtr) { unsafe { ptr.drop_as::<Value>() } }

/// Clone a runtime component by cloning the [`Value`] it holds.
///
/// A type-less component has no `TypeId`, so neither the reflect nor the
/// `Clone` handler can serve it and bevy's default is to skip it. The layout is
/// known here, though, so the clone is exact: without this an entity clone would
/// silently lose the very vocabulary a scene minted.
fn clone_value(source: &SourceComponent, cx: &mut ComponentCloneCtx) {
	// SAFETY: `DynamicComponents::register` is the only source of a dynamic
	// `ComponentId`, and it always declares a `Value` layout.
	let value = unsafe { source.ptr().deref::<Value>() }.clone();
	// the target takes ownership of the bytes copied out of `value`, so this
	// clone must not also run its destructor.
	let value = ManuallyDrop::new(value);
	// SAFETY: the pointer references a `Value`, which is the target component's
	// layout, and the target owns it from here.
	unsafe { cx.write_target_component_ptr(Ptr::from(&*value)) };
}
