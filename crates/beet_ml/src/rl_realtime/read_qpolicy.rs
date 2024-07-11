use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(read_q_policy::<P>.in_set(TickSet))]
pub struct ReadQPolicy<P: QPolicy + Asset> {
	#[reflect(ignore)]
	phantom: PhantomData<P>,
}

impl<P: QPolicy + Asset> Default for ReadQPolicy<P> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}

fn read_q_policy<P: QPolicy + Asset>(
	mut commands: Commands,
	assets: Res<Assets<P>>,
	mut agents: Query<(&P::State, &mut P::Action)>,
	query: Query<(Entity, &Handle<P>, &ReadQPolicy<P>), With<Running>>,
) {
	for (entity, handle, _read_q_policy) in query.iter() {
		if let Some(policy) = assets.get(handle) {
			for (state, mut action) in agents.iter_mut() {
				*action = policy.greedy_policy(state).0;
				commands.entity(entity).insert(RunResult::Success);
			}
		}
	}
}
