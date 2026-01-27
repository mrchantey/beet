use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::component::Mutable;
use bevy::reflect::Reflect;


pub trait RlSessionTypes: 'static + Send + Sync + Reflect {
	type State: StateSpace + Component<Mutability = Mutable>;
	type Action: ActionSpace + Component<Mutability = Mutable>;
	type QLearnPolicy: Component<Mutability = Mutable>
		+ QPolicy<State = Self::State, Action = Self::Action>;
	type Env: Environment<State = Self::State, Action = Self::Action>
		+ Component<Mutability = Mutable>;
	type EpisodeParams: EpisodeParams;
}
