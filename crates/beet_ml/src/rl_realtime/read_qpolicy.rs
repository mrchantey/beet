use crate::prelude::*;
use beet_flow::prelude::*;
use beet_core::prelude::*;
use std::marker::PhantomData;


/// Read the QPolicy from the asset and update the agent's action.
/// ## Tags
/// - [MutateOrigin](ActionTag::MutateOrigin)
#[action(read_q_policy::<P>)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
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
	ev: On<Run>,
	mut commands: Commands,
	assets: Res<Assets<P>>,
	mut agents: Query<(&P::State, &mut P::Action)>,
	query: Query<(&ReadQPolicy<P>, &HandleWrapper<P>)>,
) {
	let (_, handle) = query
		.get(ev.event_target())
		.expect(&expect_action::to_have_action(&ev));

	let policy = assets
		.get(&**handle)
		.expect(&expect_action::to_have_asset(&ev));

	let (state, mut action) = agents
		.get_mut(ev.origin)
		.expect(&expect_action::to_have_origin(&ev));


	*action = policy.greedy_policy(state).0;
	ev.trigger_result(&mut commands, RunResult::Success);
}
