//! Component types minted at runtime, with no rust definition behind them.
use beet_core::prelude::*;
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

/// Declares a component type that has no rust definition, minted when this
/// declaration is spawned.
///
/// ```ignore
/// <DynamicComponent name="guestbook.Visits"/>
/// ```
///
/// Minting vocabulary without recompiling is the point, so this is a component a
/// scene keeps rather than a template that expands away: a scene that declares
/// `guestbook.Visits` still declares it after a serde round trip.
///
/// The value is a [`Value`], the same thing a document field holds, so a runtime
/// component carries whatever JSON can: a count, a flag, a string, a list, a
/// map. That is enough to give a scene words the engine never shipped.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(DynamicComponent::mint))]
pub struct DynamicComponent {
	/// The name the component is addressed by, in a scene and in a script.
	pub name: SmolStr,
}

impl DynamicComponent {
	/// Declare a runtime component named `name`.
	pub fn new(name: impl Into<SmolStr>) -> Self { Self { name: name.into() } }

	/// The hook body: register the declared name once the command queue flushes.
	///
	/// Registration is structural, which a [`DeferredWorld`](bevy::ecs::world::DeferredWorld)
	/// cannot do, so it is queued rather than performed in the hook.
	fn mint(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		let name = self.name.clone();
		move |entity: &mut EntityCommands| {
			entity.commands().queue(move |world: &mut World| {
				DynamicComponents::register(world, &name);
			});
		}
	}
}

/// The runtime component vocabulary this world has minted, mapping each declared
/// name to its live [`ComponentId`].
///
/// Bevy mints a *distinct* component for every descriptor it is handed, even
/// identical ones, so the name is the identity and this map is what keeps a
/// second declaration of the same name from minting a second component.
#[derive(Debug, Default, Resource)]
pub struct DynamicComponents {
	by_name: HashMap<SmolStr, ComponentId>,
	by_id: HashMap<ComponentId, SmolStr>,
}

impl DynamicComponents {
	/// The component registered under `name`, if any.
	pub fn get(world: &World, name: &str) -> Option<ComponentId> {
		world
			.get_resource::<Self>()
			.and_then(|this| this.by_name.get(name).copied())
	}

	/// Register a runtime component named `name`, returning the existing one if
	/// the name is already minted.
	pub fn register(world: &mut World, name: &str) -> ComponentId {
		if let Some(id) = Self::get(world, name) {
			return id;
		}
		// SAFETY: the layout is `Value`, which is `Send + Sync`, so the
		// descriptor's unconditional `is_send_and_sync` holds, and `drop_value`
		// matches it. Every write to this component goes through
		// `WorldWrite::insert`, which writes exactly one owned `Value`.
		let descriptor = unsafe {
			ComponentDescriptor::new_with_layout(
				name.to_string(),
				StorageType::Table,
				Layout::new::<Value>(),
				Some(drop_value),
				true,
				ComponentCloneBehavior::Custom(clone_value),
				None,
			)
		};
		let id = world.register_component_with_descriptor(descriptor);
		let mut this = world.get_resource_or_init::<Self>();
		this.by_name.insert(name.into(), id);
		this.by_id.insert(id, name.into());
		id
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn registering_twice_yields_one_component() {
		let mut world = World::new();
		let first = DynamicComponents::register(&mut world, "game.Health");
		let second = DynamicComponents::register(&mut world, "game.Health");
		first.xpect_eq(second);
	}

	#[beet_core::test]
	fn distinct_names_are_distinct_components() {
		let mut world = World::new();
		let health = DynamicComponents::register(&mut world, "game.Health");
		let mana = DynamicComponents::register(&mut world, "game.Mana");
		(health == mana).xpect_false();
	}

	#[beet_core::test]
	fn a_declaration_mints_its_component() {
		let mut world = World::new();
		world.spawn(DynamicComponent::new("game.Health"));
		world.flush();
		DynamicComponents::get(&world, "game.Health")
			.is_some()
			.xpect_true();
	}

	/// Insert a runtime component through the bridge and read it back, the round
	/// trip every script value takes.
	fn round_trip(json: serde_json::Value) -> serde_json::Value {
		let mut world = test_world();
		DynamicComponents::register(&mut world, "game.Loot");
		let entity = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			entity,
			"game.Loot",
			&json,
			&ScriptExposure::default(),
		)
		.unwrap();
		WorldRead::get(
			&mut world,
			entity,
			"game.Loot",
			&ScriptExposure::default(),
		)
		.unwrap()
		.unwrap()
	}

	/// A runtime component holds anything a document field can, not just a
	/// number: the heap-allocating cases are the ones the layout's drop fn has
	/// to get right.
	#[beet_core::test]
	fn round_trips_a_string() {
		round_trip(serde_json::json!("sword"))
			.xpect_eq(serde_json::json!("sword"));
	}

	#[beet_core::test]
	fn round_trips_a_list() {
		round_trip(serde_json::json!(["sword", "shield"]))
			.xpect_eq(serde_json::json!(["sword", "shield"]));
	}

	#[beet_core::test]
	fn round_trips_a_map() {
		round_trip(serde_json::json!({ "sword": 2 }))
			.xpect_eq(serde_json::json!({ "sword": 2 }));
	}

	/// A type-less component has nothing for the reflect clone handler to work
	/// from, so the descriptor carries its own: without it an entity clone would
	/// silently drop the runtime vocabulary.
	#[beet_core::test]
	fn cloning_an_entity_clones_the_value() {
		let mut world = test_world();
		DynamicComponents::register(&mut world, "game.Loot");
		let source = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			source,
			"game.Loot",
			&serde_json::json!(["sword"]),
			&ScriptExposure::default(),
		)
		.unwrap();
		let target = world.entity_mut(source).clone_and_spawn();
		WorldRead::get(
			&mut world,
			target,
			"game.Loot",
			&ScriptExposure::default(),
		)
		.unwrap()
		.unwrap()
		.xpect_eq(serde_json::json!(["sword"]));
	}
}
