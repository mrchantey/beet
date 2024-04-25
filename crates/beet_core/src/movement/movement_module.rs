use super::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Default, Clone, BeetModule)]
#[actions(
	Hover,
	Translate,
	SetAgentOnRun::<Velocity>,
)]
#[components(Mass, Velocity, Impulse, Force)]
#[bundles(ForceBundle)]
/// Actions related to movement and basic physics
pub struct MovementModule;
