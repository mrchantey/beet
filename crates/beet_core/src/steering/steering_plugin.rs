use crate::prelude::*;
use bevy_app::prelude::*;
#[allow(unused)]
use bevy_time::Time;


/// Required Resources:
/// - [`Time`]
#[derive(Default)]
pub struct SteeringPlugin;


impl Plugin for SteeringPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PostUpdate, integrate_force);
	}
}
