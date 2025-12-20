use crate::prelude::*;
use beet_core::prelude::*;
// /// A number generator for determining.
// rng: StdRng,
// /// Whether there is a 2/3 chance the agent moves left or right of the intended direction.
// is_slippery: bool,

#[derive(Clone, Component)]
/// An environment for the Frozen Lake game.
pub struct QTableEnv<S: StateSpace + Clone, A: ActionSpace + Clone> {
	/// The transition probabilities for each state-action pair.
	outcomes: HashMap<(S, A), StepOutcome<S>>,
}

impl<S: StateSpace + Clone, A: ActionSpace + Clone> QTableEnv<S, A> {
	pub fn new(outcomes: HashMap<(S, A), StepOutcome<S>>) -> Self {
		Self { outcomes }
	}
}

impl<S: StateSpace + Clone, A: ActionSpace + Clone> Environment
	for QTableEnv<S, A>
{
	type State = S;
	type Action = A;

	fn step(
		&mut self,
		state: &Self::State,
		action: &Self::Action,
	) -> StepOutcome<Self::State> {
		self.outcomes[&(state.clone(), action.clone())].clone()
	}
}
