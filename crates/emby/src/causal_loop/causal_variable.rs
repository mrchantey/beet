use serde::Deserialize;
use serde::Serialize;
use bevy::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Component)]
pub struct CausalVariable {
	name: String,
	/// The unicode for the emoji
	emoji: String,
	value: f32,
	min_value: f32,
	max_value: f32,
	description: String,
}
