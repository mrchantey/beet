use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::change_detection::MaybeLocation;
use bevy::ecs::world::CommandQueue;
use std::fmt;



pub struct ActionContext {
	/// The [`Entity`] this event is currently triggered for.
	pub action: Entity,
	/// The 'agent' entity for this action.
	/// Unless explicitly specified the agent is the first [`ActionOf`] in the
	/// action's ancestors (inclusive), or the root ancestor if no [`ActionOf`]
	/// is found.
	pub agent: Entity,
	pub(super) queue: CommandQueue,
}

impl ActionContext {
	pub fn new(action_entity: &mut EntityWorldMut) -> Self {
		let action = action_entity.id();

		let agent = action_entity
			.world_scope(move |world| {
				world.flush();
				world.run_system_once(
					move |parents: Query<&ChildOf>,
					      actions: Query<&ActionOf>| {
						parents
							.iter_ancestors_inclusive(action)
							.find_map(|entity| {
								actions.get(entity).ok().map(|a| a.get())
							})
							.unwrap_or(parents.root_ancestor(action))
					},
				)
			})
			.unwrap_or(action);

		Self {
			action,
			agent,
			queue: default(),
		}
	}

	pub fn new_with_agent(action: Entity, agent: Entity) -> Self {
		Self {
			action,
			agent,
			queue: default(),
		}
	}

	/// Get the current action [`Entity`]
	pub fn action(&self) -> Entity { self.action }

	/// Get the [`ActionContext::agent`] entity
	pub fn agent(&self) -> Entity { self.agent }

	#[track_caller]
	pub fn trigger_next(&mut self, event: impl ActionEvent) -> &mut Self {
		self.trigger_next_with(self.action, event)
	}
	#[track_caller]
	pub fn trigger_next_with(
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

	#[track_caller]
	pub fn trigger_target<M>(
		&mut self,
		ev: impl IntoEntityTargetEvent<M>,
	) -> &mut Self {
		let action = self.action;
		let caller = MaybeLocation::caller();
		self.queue.push(move |world: &mut World| {
			let mut entity = world.entity_mut(action);
			let (mut ev, mut trigger) =
				ev.into_entity_target_event(&mut entity);
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
	use sweet::prelude::*;

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
			.spawn(children![(
				set_agent(store),
				OnSpawn::trigger(Run),
				// allowed to add after OnSpawn::trigger?
				ActionOf(agent),
			)])
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
