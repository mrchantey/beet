use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct ReadQPolicy<P: QPolicy + Asset> {
	pub policy_handle: Handle<P>,
}

impl<P: QPolicy + Asset> ReadQPolicy<P> {
	pub fn new(table: Handle<P>) -> Self {
		Self {
			policy_handle: table,
		}
	}
}

fn read_q_policy<P: QPolicy + Asset>(
	mut commands: Commands,
	assets: Res<Assets<P>>,
	mut agents: Query<(&P::State, &mut P::Action)>,
	query: Query<(Entity, &ReadQPolicy<P>), With<Running>>,
) {
	for (entity, read_q_policy) in query.iter() {
		if let Some(policy) = assets.get(&read_q_policy.policy_handle) {
			for (state, mut action) in agents.iter_mut() {
				*action = policy.greedy_policy(state).0;
				commands.entity(entity).insert(RunResult::Success);
			}
		}
	}
}

impl<P: QPolicy + Asset> ActionMeta for ReadQPolicy<P> {
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl<P: QPolicy + Asset> ActionSystems for ReadQPolicy<P> {
	fn systems() -> SystemConfigs { read_q_policy::<P>.in_set(TickSet) }
}
