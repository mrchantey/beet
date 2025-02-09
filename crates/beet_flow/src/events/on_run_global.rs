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
		.add_observer(bubble_run_result_global)
		;
}

/// Tracks action observers created when the first action is added to a node.
#[derive(Debug, Default, Resource)]
pub struct ActionMap {
	/// All of the actions that should be triggered
	/// when a given tree entity runs
	// TODO entity relations 0.16
	pub node_to_actions: HashMap<Entity, Vec<Entity>>,
	/// TODO add marker to the observer entity instead
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
#[derive(Debug, Copy, Clone, Event, Deref, DerefMut)]
pub struct OnRunGlobal(pub RunContext);

/// Trigger [OnRunGlobal] on a given entity.
impl Default for OnRunGlobal {
	fn default() -> Self { Self(RunContext::placeholder()) }
}
impl From<RunContext> for OnRunGlobal {
	fn from(context: RunContext) -> Self { Self(context) }
}


#[derive(Debug, Clone, Copy, PartialEq, Hash, Reflect)]
pub struct RunContext {
	/// aka `agent`, this is the entity that is being targetd by this action.
	/// In most patterns this lies at the root of the tree, but in the case of
	/// shared trees this can be some arbitrary entity.
	pub target: Entity,
	/// The current node of the tree that is running
	pub action: Entity,
}

impl RunContext {
	pub fn placeholder() -> Self {
		Self {
			target: Entity::PLACEHOLDER,
			action: Entity::PLACEHOLDER,
		}
	}
}

impl RunContext {}

pub trait HasRunContext {
	/// Trigger [OnRun] for a target entity that is also
	/// the node
	fn with_action(action: Entity) -> Self;
	/// Trigger [OnRun] for a target entity and node
	fn with_target_and_action(action: Entity, target: Entity) -> Self;
}

impl<T: From<RunContext>> HasRunContext for T {
	/// Called via `world.trigger()`
	fn with_action(action: Entity) -> Self {
		RunContext {
			action,
			target: action,
		}
		.into()
	}

	fn with_target_and_action(action: Entity, target: Entity) -> Self {
		RunContext { target, action }.into()
	}
}


/// A general observer triggered globally that can be mapped to specific actions.
// This is intentionally not generic with a value:T because that would
// involve cloning the data which is an ecs antipattern
#[derive(Debug, Copy, Clone, Event, Deref, DerefMut)]
pub struct OnAction(pub RunContext);

impl From<RunContext> for OnAction {
	fn from(context: RunContext) -> Self { Self(context) }
}

impl OnAction {
	pub fn into_result(self, result: RunResult) -> OnRunResultGlobal {
		OnRunResultGlobal::new(self.0, result)
	}
}


// #[derive(SystemParam)]
// struct RunChild<'w, 's> {
// 	commands: Commands<'w, 's>,
// 	children: Query<'w, 's, &'static Children>,
// }

// impl<'w, 's> RunChild<'w, 's> {
// 	pub fn run_child(&mut self, entity: Entity) {}
// }

#[extend::ext(name=ActionTrigger)]
pub impl<'a> Trigger<'a, OnAction> {
	fn run_next<'w, 's>(&self, mut commands: Commands<'w, 's>, action: Entity) {
		commands.entity(action).trigger(OnRunGlobal(
			RunContext::with_target_and_action(action, self.target),
		));
	}
	fn on_result<'w, 's>(
		&self,
		mut commands: Commands<'w, 's>,
		result: RunResult,
	) {
		commands
			.entity(self.action)
			.trigger(self.event().into_result(result));
	}
}

/// Global observer to call OnRun for each action registered
/// on the action entity.
///
/// # Panics
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

	let target = if trigger.target == Entity::PLACEHOLDER {
		action
	} else {
		trigger.target
	};


	if let Some(actions) = action_map.node_to_actions.get(&action) {
		let on_action: OnAction = RunContext { target, action }.into();
		commands.trigger_targets(on_action, actions.clone());
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
			.flush_trigger(OnRunGlobal::default())
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
