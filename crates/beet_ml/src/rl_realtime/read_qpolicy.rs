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
#[action(plain_meta)]
#[derive(Component, Reflect)]
#[reflect(Component, Default)]
pub fn ReadQPolicy<P>(
	/// Asset handle for the policy to read.
	#[field]
	handle: Handle<P>,
	cx: In<ActionContext>,
	assets: Res<Assets<P>>,
	mut agents: AgentQuery<(&P::State, &mut P::Action)>,
) -> Result<Outcome>
where
	P: QPolicy + Asset,
	P::State: Component,
	P::Action: Component,
{
	let action_entity = cx.caller.id();
	let policy = assets.get(&handle).ok_or_else(|| {
		bevyhow!("QPolicy asset not loaded for entity {:?}", action_entity)
	})?;
	let (state, mut action) = agents.get_mut(action_entity)?;
	*action = policy.greedy_policy(state).0;
	Ok(Outcome::PASS)
}

impl<P: QPolicy + Asset> ReadQPolicy<P> {
	/// Create a [`ReadQPolicy`] from an asset handle.
	pub fn new(handle: Handle<P>) -> Self {
		Self {
			handle,
			_marker: PhantomData,
		}
	}
}
