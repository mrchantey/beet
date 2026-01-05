use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::world::CommandQueue;
use std::fmt;


pub struct ActionContext {
	/// The [`Action`] entity this event is currently triggered for
	pub action: Entity,
	/// The 'agent' entity for this action.
	/// Unless explicitly specified the agent is the first [`ActionOf`] in the
	/// action's ancestors (inclusive), or the root ancestor if no [`ActionOf`]
	/// is found.
	pub agent: Entity,
	pub(super) queue: CommandQueue,
}

impl Clone for ActionContext {
	fn clone(&self) -> Self {
		Self {
			action: self.action,
			agent: self.agent,
			queue: default(),
		}
	}
}

impl ActionContext {
	/// only for use by ActionEvent which will immediately
	/// set with new_find_agent if the `action` is Entity::PLACEHOLDER
	pub(super) fn new_no_agent(action: Entity) -> Self {
		Self {
			action,
			agent: Entity::PLACEHOLDER,
			queue: default(),
		}
	}
	/// Use the hierarchy and [`ActionOf`] components to infer the
	/// agent for this action.
	// TODO may need to deprecate with bevy auto propagate components
	pub(super) fn find_agent(&self, world: &World) -> Entity {
		// first check for an ActionOf on the action entity directly
		if let Some(action_of) = world.entity(self.action).get::<ActionOf>() {
			return action_of.get();
		}
		// othwerwise visit ancestors
		let mut agent = self.action;
		while let Some(parent) = world.entity(agent).get::<ChildOf>() {
			// first check if the current agent has an action
			if let Some(action_of) =
				world.entity(parent.get()).get::<ActionOf>()
			{
				agent = action_of.get();
				break;
			} else {
				// otherwise move up the tree
				agent = parent.get();
			}
		}
		agent
	}

	pub fn new_with_agent(action: Entity, agent: Entity) -> Self {
		Self {
			action,
			agent,
			queue: default(),
		}
	}

	/// Get the current [`Action`] entity
	pub fn action(&self) -> Entity { self.action }

	/// Get the current [`Agent`] entity
	pub fn agent(&self) -> Entity { self.agent }

	/// Trigger the event on this [`Action`] with this action's context.
	#[track_caller]
	pub fn trigger_with_cx(&mut self, event: impl ActionEvent) -> &mut Self {
		self.trigger_action_with_cx(self.action, event)
	}

	/// Trigger the event with the provided [`Action`] with this action's context.
	#[track_caller]
	pub fn trigger_action_with_cx(
		&mut self,
		action: Entity,
		mut event: impl ActionEvent,
	) -> &mut Self {
		let cx = ActionContext::new_with_agent(action, self.agent);
		let caller = MaybeLocation::caller();
		self.queue.push(move |world: &mut World| {
			let mut trigger = ActionTrigger::new(cx);
			world.trigger_ref_with_caller_pub(&mut event, &mut trigger, caller);
		});
		self
	}

	/// Run an async function from this action with a provided [`AsyncAction`]
	#[track_caller]
	pub fn run_async<Func, Fut, Out>(&mut self, func: Func) -> &mut Self
	where
		Func: 'static + Send + FnOnce(AsyncAction) -> Fut,
		Fut: MaybeSend + Future<Output = Out>,
		Out: AsyncTaskOut,
	{
		let cx = self.clone();
		self.queue.push(move |world: &mut World| {
			world.run_async(async move |world| {
				// Wrap the context in an AsyncAction helper that applies its queue on drop.
				let async_entity = world.entity(cx.action());
				let async_action = AsyncAction::new(async_entity, cx);
				func(async_action).await;
				// async_action dropped here; its Drop applies any queued commands
			});
		});
		self
	}

	#[track_caller]
	pub fn trigger_target<M>(
		&mut self,
		ev: impl IntoEntityTargetEvent<M>,
	) -> &mut Self {
		let action = self.action;
		let caller = MaybeLocation::caller();
		self.queue.push(move |world: &mut World| {
			let (mut ev, mut trigger) = ev.into_entity_target_event(action);
			world.trigger_ref_with_caller_pub(&mut ev, &mut trigger, caller);
		});
		self
	}
}


impl fmt::Debug for ActionContext {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ActionContext")
			.field("action", &self.action)
			.field("agent", &self.agent)
			.finish()
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[derive(ActionEvent)]
	struct Run;

	fn set_agent(store: Store<Entity>) -> impl Bundle {
		OnSpawn::observe(move |ev: On<Run>| {
			store.set(ev.agent());
		})
	}


	#[test]
	fn agent_is_action() {
		let mut world = World::new();
		let store = Store::new(Entity::PLACEHOLDER);
		let action = world
			.spawn(set_agent(store))
			.trigger_target(Run)
			.flush()
			.id();
		store.get().xpect_eq(action);
	}
	#[test]
	fn agent_is_root() {
		let mut world = World::new();
		let store = Store::new(Entity::PLACEHOLDER);
		let root = world
			.spawn(children![(set_agent(store), OnSpawn::trigger(Run))])
			.flush();
		store.get().xpect_eq(root);
	}
	#[test]
	fn agent_is_action_of() {
		let mut world = World::new();
		let store = Store::new(Entity::PLACEHOLDER);
		let agent = world.spawn_empty().id();
		world
			.spawn((
				// allowed to add after OnSpawn::trigger?
				ActionOf(agent),
				set_agent(store),
			))
			.trigger_target(Run)
			.flush();
		store.get().xpect_eq(agent);
	}
	#[test]
	fn agent_is_explicit_set() {
		let mut world = World::new();
		let store = Store::new(Entity::PLACEHOLDER);
		let agent = world.spawn_empty().id();
		world
			.spawn(children![(
				set_agent(store),
				OnSpawn::trigger(Run.with_agent(agent))
			)])
			.flush();
		store.get().xpect_eq(agent);
	}
}
