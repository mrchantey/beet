use crate::prelude::*;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
#[allow(unused)]
use bevy_time::Time;
#[allow(unused)]
use bevy_transform::prelude::TransformBundle;


/// Required Resources:
/// - [`Time`]
#[derive(Default)]
pub struct SteeringPlugin;


impl Plugin for SteeringPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PostUpdate, integrate_force);
	}
}

/// This should be used in conjunction with the [`ForceBundle`] and [`TransformBundle`]
#[derive(Default, Bundle)]
pub struct SteerBundle {
	pub max_force: MaxForce,
	pub max_speed: MaxSpeed,
	pub arrive_radius: ArriveRadius,
	pub wander_params: WanderParams,
}

impl SteerBundle {
	pub fn with_target(self, target: impl Into<SteerTarget>) -> impl Bundle {
		// self.steer_target = target.into();
		(self, target.into())
	}
}
