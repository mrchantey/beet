use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::utils::HashMap;


#[rustfmt::skip]
pub fn on_run_global_plugin(app: &mut App) {
	app
		.init_resource::<ActionMap>()
		.add_observer(on_run_global)
		;
}

/// Tracks action observers created when the first action is added to a node.
#[derive(Debug, Default, Resource)]
pub struct ActionMap {
	/// All of the actions that should be triggered
	/// when a given tree entity runs
	// TODO entity relations 0.16
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
pub struct OnRunGlobal {
	/// The entity targeted by the behavior
	pub target: Entity,
	/// The entity containing the actions to perform
	pub action: Entity,
}

impl OnRunGlobal {
	/// Trigger [OnRunGlobal] on a given entity.
	pub fn new() -> Self {
		Self {
			target: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
	/// Trigger [OnRun] for a target entity that is also
	/// the node
	pub fn with_action(action: Entity) -> Self {
		Self {
			action,
			target: action,
		}
	}
	/// Trigger [OnRun] for a target entity and node
	pub fn with_target_and_action(action: Entity, target: Entity) -> Self {
		Self { target, action }
	}
}

/// A general observer triggered globally that can be mapped to specific actions.
// This is intentionally not generic with a value:T because that would
// involve cloning the data which is an ecs antipattern
#[derive(Debug, Copy, Clone, Event)]
pub struct OnAction {
	/// The entity targeted by the behavior
	pub target: Entity,
	/// The entity containing the actions to perform
	pub action: Entity,
}


/// Call OnRun for each action registered
///
/// # Panics
///
/// If the trigger does specify an action, usually because
/// `OnRun` was called directly without `with_target`
pub fn on_run_global(
	trigger: Trigger<OnRunGlobal>,
	action_map: Res<ActionMap>,
	mut commands: Commands,
) {
	let action = if trigger.action == Entity::PLACEHOLDER {
		let trigger_entity = trigger.entity();
		if trigger_entity == Entity::PLACEHOLDER {
			panic!("{}", expect_run::to_have_node(trigger.event()));
		}
		trigger_entity
	} else {
		trigger.action
	};


	if let Some(actions) = action_map.node_to_actions.get(&action) {
		let action = OnAction {
			target: trigger.target,
			action,
		};
		commands.trigger_targets(action, actions.clone());
	}
}


#[cfg(test)]
mod test {
	use beet_flow::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[derive(Default, GlobalAction)]
	#[observers(trigger_count)]
	struct TriggerCount(i32);

	fn trigger_count(
		trigger: Trigger<OnAction>,
		mut query: Query<&mut TriggerCount>,
	) {
		query.get_mut(trigger.action).unwrap().as_mut().0 += 1;
	}

	#[test]
	fn inferred_action() {
		let mut app = App::new();

		app.add_plugins(on_run_global_plugin);

		let entity = app
			.world_mut()
			.spawn(TriggerCount::default())
			.flush_trigger(OnRunGlobal::new())
			.id();
		expect(app.world().get::<TriggerCount>(entity).unwrap().0).to_be(1);
	}
	#[test]
	fn explicit_action() {
		let mut app = App::new();

		app.add_plugins(on_run_global_plugin);

		let entity = app.world_mut().spawn(TriggerCount::default()).id();

		app.world_mut()
			.flush_trigger(OnRunGlobal::with_action(entity));

		expect(app.world().get::<TriggerCount>(entity).unwrap().0).to_be(1);
	}
}
