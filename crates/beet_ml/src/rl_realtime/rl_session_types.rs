use crate::prelude::*;
use bevy::reflect::Reflect;



pub trait RlSessionTypes: 'static + Send + Sync + Reflect {
	type State: StateSpace;
	type Action: ActionSpace;
	type QLearnPolicy: QPolicy<State = Self::State, Action = Self::Action>;
	type Env: Environment<State = Self::State, Action = Self::Action>;
	type EpisodeParams: EpisodeParams;
}
