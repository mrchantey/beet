use crate::prelude::*;
use bevy::prelude::*;




#[derive(Bundle)]
pub struct RlAgentBundle<Env: Component + Environment> {
	pub state: Env::State,
	pub action: Env::Action,
	pub env: Env,
	pub params: QLearnParams,
	pub session: SessionEntity,
	pub despawn:DespawnOnEpisodeEnd
}

// #[derive(Bundle)]
// pub struct RlSessionBundle<S: RlSessionTypes>
// where
// 	S::QSource: Component,
// {
// 	pub source: S::QSource,
// }
