use beet_core::prelude::*;

/// The reward most recently observed by the agent in this episode.
#[derive(Component)]
pub struct Reward(pub f32);

/// The episode index the agent is participating in.
#[derive(Component)]
pub struct Episode(pub f32);
