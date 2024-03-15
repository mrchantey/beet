use crate::prelude::*;
use beet_ecs::action::PostTickSet;
use bevy::prelude::*;
use forky_bevy::extensions::AppExt;

/// Required Resources:
/// - [`Time`]
#[derive(Default)]
pub struct SteeringPlugin;


impl Plugin for SteeringPlugin {
	fn build(&self, app: &mut App) {
		app.__()
			.add_systems(
				Update,
				(integrate_force, wrap_around).chain().in_set(PostTickSet),
			)
			.insert_resource(WrapAround::default())
			.__();
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
