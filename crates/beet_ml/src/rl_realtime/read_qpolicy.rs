use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use std::marker::PhantomData;


/// Read the QPolicy from the asset and update the agent's action.
/// ## Tags
/// - [MutateAgent](ActionTag::MutateAgent)
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
	ev: On<GetOutcome>,
	mut commands: Commands,
	assets: Res<Assets<P>>,
	mut agents: AgentQuery<(&P::State, &mut P::Action)>,
	query: Query<(&ReadQPolicy<P>, &HandleWrapper<P>)>,
) -> Result {
	let action_entity = ev.target();
	let (_, handle) = query.get(action_entity)?;
	let policy = assets.get(&**handle).ok_or_else(|| {
		bevyhow!("QPolicy asset not loaded for entity {:?}", action_entity)
	})?;

	let (state, mut action) = agents.get_mut(action_entity)?;

	*action = policy.greedy_policy(state).0;
	commands.entity(action_entity).trigger_target(Outcome::Pass);
	Ok(())
}
