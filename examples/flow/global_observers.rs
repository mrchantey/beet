#![allow(dead_code)]
//! An example of the general pattern used by beet in vanilla bevy
//! Hopefully this makes how beet works a bit clearer
use bevy::ecs::component::ComponentId;
use bevy::ecs::component::StorageType;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::utils::HashMap;

#[derive(Default)]
struct TriggerCount(i32);

impl Component for TriggerCount {
	const STORAGE_TYPE: StorageType = StorageType::Table;
	fn register_component_hooks(
		hooks: &mut bevy::ecs::component::ComponentHooks,
	) {
		hooks.on_add(|mut world, node, cid| {
			ActionMap::on_add(&mut world, node, cid, |world, action| {
				world.commands().entity(action).observe(trigger_count);
			});
		});
		hooks.on_remove(|mut world, node, _cid| {
			ActionMap::on_remove(&mut world, node);
		});
	}
}


fn trigger_count(
	trigger: Trigger<OnAction>,
	mut query: Query<&mut TriggerCount>,
) {
	query.get_mut(trigger.node).unwrap().as_mut().0 += 1;
}



fn main() {
	let mut app = App::new();

	// these would both be automatically added
	// when the first [Health] is spawned
	app.init_resource::<ActionMap>().add_observer(on_run);

	let start = std::time::Instant::now();
	let entity = app.world_mut().spawn(TriggerCount::default()).id();
	app.world_mut().flush();
	app.world_mut().trigger(OnRun::new(entity));
	app.world_mut().flush();
	for _ in 0..10_u64.pow(6) {
		let entity = app.world_mut().spawn(TriggerCount::default()).id();

		app.world_mut().flush();
		app.world_mut().trigger(OnRun::new(entity));
		app.world_mut().flush();
		assert_eq!(app.world().get::<TriggerCount>(entity).unwrap().0, 1);
	}
	println!("Time: {}", start.elapsed().as_millis());
	// 600ms
}

/// Map
#[derive(Debug, Default, Resource)]
struct ActionMap {
	/// All of the actions that should be triggered
	/// when a given tree entity runs
	pub node_to_actions: HashMap<Entity, Vec<Entity>>,
	pub cid_to_action: HashMap<ComponentId, Entity>,
}

impl ActionMap {
	/// Called by an actions component hooks when it is added to
	/// a node.
	pub fn on_add(
		world: &mut DeferredWorld,
		node: Entity,
		cid: ComponentId,
		on_create_action: impl FnMut(&mut DeferredWorld, Entity),
	) {
		let action = Self::get_or_insert_action(world, cid, on_create_action);
		let mut map = world.resource_mut::<ActionMap>();
		map.node_to_actions
			.entry(node)
			.or_insert_with(Vec::new)
			.push(action);
	}
	/// Called by an actions component hooks when it is removed from
	/// a node.
	pub fn on_remove(world: &mut DeferredWorld, node: Entity) {
		let mut map = world.resource_mut::<ActionMap>();
		if let Some(actions) = map.node_to_actions.get_mut(&node) {
			actions.retain(|&e| e != node);
		}
	}

	/// get the Action entity for a given type
	fn get_or_insert_action(
		world: &mut DeferredWorld,
		cid: ComponentId,
		mut on_create_action: impl FnMut(&mut DeferredWorld, Entity),
	) -> Entity {
		let map = world.resource::<ActionMap>();
		if let Some(action) = map.cid_to_action.get(&cid) {
			return *action;
		}
		let action = world.commands().spawn_empty().id();
		on_create_action(world, action);
		let mut map = world.resource_mut::<ActionMap>();
		map.cid_to_action.insert(cid, action);
		action
	}
}


/// A general observer triggered globally that can be mapped to specific actions.
#[derive(Debug, Copy, Clone, Event)]
struct OnRun {
	/// The entity targeted by the behavior
	pub target: Entity,
	/// The entity containing the actions to perform
	pub node: Entity,
}

impl OnRun {
	/// Trigger [OnRun] for a target entity
	/// that will also
	pub fn new(target: Entity) -> Self {
		Self {
			target: target.clone(),
			node: target,
		}
	}
	/// Trigger [OnRun] for a target entity
	/// that will also
	pub fn new_with_tree(target: Entity, tree: Entity) -> Self {
		Self { target, node: tree }
	}
	fn into_action(self) -> OnAction {
		OnAction {
			target: self.target,
			node: self.node,
		}
	}
}

/// A general observer triggered globally that can be mapped to specific actions.
#[derive(Debug, Copy, Clone, Event)]
struct OnAction {
	/// The entity targeted by the behavior
	pub target: Entity,
	/// The entity containing the actions to perform
	pub node: Entity,
}


/// Call OnRun for each action registered by this entity
fn on_run(
	trigger: Trigger<OnRun>,
	action_map: Res<ActionMap>,
	mut commands: Commands,
) {
	if let Some(actions) = action_map.node_to_actions.get(&trigger.node) {
		let action: OnAction = trigger.event().into_action();
		commands.trigger_targets(action, actions.clone());
	}
}
