use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use std::marker::PhantomData;

/// One-shot action: looks up the agent's current state in a [`QPolicy`]
/// asset and writes the greedy action onto the agent before returning
/// [`Outcome::PASS`].
///
/// The [`Handle`] lives on the action component itself rather than via a
/// wrapper, since the action struct already derives [`Component`].
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(Action<(), Outcome> = Action::<(), Outcome>::new_system(read_q_policy::<P>))]
pub struct ReadQPolicy<P: QPolicy + Asset> {
	/// Asset handle for the policy to read.
	pub handle: Handle<P>,
	#[reflect(ignore)]
	phantom: PhantomData<P>,
}

impl<P: QPolicy + Asset> ReadQPolicy<P> {
	/// Create a [`ReadQPolicy`] from an asset handle.
	pub fn new(handle: Handle<P>) -> Self {
		Self {
			handle,
			phantom: PhantomData,
		}
	}
}

/// Backing system: reads the policy and writes the greedy action onto
/// the agent. Resolves [`Outcome::PASS`] on success.
fn read_q_policy<P>(
	cx: In<ActionContext>,
	assets: Res<Assets<P>>,
	mut agents: AgentQuery<(&P::State, &mut P::Action)>,
	query: Query<&ReadQPolicy<P>>,
) -> Result<Outcome>
where
	P: QPolicy + Asset,
	P::State: Component,
	P::Action: Component,
{
	let action_entity = cx.caller.id();
	let read = query.get(action_entity)?;
	let policy = assets.get(&read.handle).ok_or_else(|| {
		bevyhow!("QPolicy asset not loaded for entity {:?}", action_entity)
	})?;
	let (state, mut action) = agents.get_mut(action_entity)?;
	*action = policy.greedy_policy(state).0;
	Ok(Outcome::PASS)
}
