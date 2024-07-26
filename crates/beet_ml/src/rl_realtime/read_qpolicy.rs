use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(read_q_policy::<P>)]
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
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	assets: Res<Assets<P>>,
	mut agents: Query<(&P::State, &mut P::Action)>,
	query: Query<(&ReadQPolicy<P>, &Handle<P>, &TargetAgent)>,
) {
	let (_, handle, agent) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let policy = assets.get(handle).expect(expect_asset::NOT_READY);

	let (state, mut action) = agents
		.get_mut(agent.0)
		.expect(expect_action::TARGET_MISSING);


	*action = policy.greedy_policy(state).0;
	// log::info!("ReadQPolicy: \n{:?}\n{:?}", state, action);
	commands
		.entity(trigger.entity())
		.trigger(OnRunResult::success());
}
