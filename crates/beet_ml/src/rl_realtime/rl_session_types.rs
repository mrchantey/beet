use crate::prelude::*;



pub trait RlSessionTypes: 'static + Send + Sync {
	type State: StateSpace;
	type Action: ActionSpace;
	type QLearnPolicy: QPolicy<State = Self::State, Action = Self::Action>;
	type Env: Environment<State = Self::State, Action = Self::Action>;
	type EpisodeParams: EpisodeParams;
}
