#[allow(unused)]
use crate::prelude::*;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;


#[derive(Component)]
pub struct Reward(pub f32);

#[derive(Component)]
pub struct Episode(pub f32);

/// Random number generator for reinforcement learning.
/// This is initialized with [`SeedableRng::from_entropy`] in plugins that rely on it
/// like the [`FrozenLakePlugin`]. For testing it should be set with [`SeedableRng::seed_from_u64`]
/// to ensure deterministic results.
#[derive(Resource, Deref, DerefMut)]
pub struct RlRng(pub StdRng);

impl RlRng {
	pub fn deterministic() -> Self { Self(StdRng::seed_from_u64(0)) }
	pub fn entropy() -> Self { Self(StdRng::from_entropy()) }
}

impl Default for RlRng {
	fn default() -> Self { Self(StdRng::from_entropy()) }
}
