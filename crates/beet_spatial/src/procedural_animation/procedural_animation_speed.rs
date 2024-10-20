use bevy::prelude::*;


#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Debug, PartialEq, Default)]
pub enum ProceduralAnimationSpeed {
	MetersPerSecond(f32),
	FractionPerSecond(f32),
}

impl Default for ProceduralAnimationSpeed {
	fn default() -> Self { Self::MetersPerSecond(1.0) }
}
