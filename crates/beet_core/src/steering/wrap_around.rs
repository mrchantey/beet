use beet_ecs::node::AgentMarker;
use bevy::prelude::*;

pub const DEFAULT_WRAPAROUND_HALF_EXTENTS: f32 = 1.;


#[derive(Debug, Clone, Resource)]
pub struct WrapAround {
	pub half_extents: Vec3,
}

impl Default for WrapAround {
	fn default() -> Self {
		Self {
			half_extents: Vec3::splat(DEFAULT_WRAPAROUND_HALF_EXTENTS),
		}
	}
}

impl WrapAround {
	pub fn cube(half_width: f32) -> Self {
		Self {
			half_extents: Vec3::new(half_width, half_width, half_width),
		}
	}
}

pub fn wrap_around(
	wrap: Res<WrapAround>,
	mut query: Query<&mut Transform, (With<AgentMarker>, Changed<Transform>)>,
) {
	for mut transform in query.iter_mut() {
		if transform.translation.x > wrap.half_extents.x {
			transform.translation.x -= wrap.half_extents.x * 2.;
		} else if transform.translation.x < -wrap.half_extents.x {
			transform.translation.x += wrap.half_extents.x * 2.;
		}
		if transform.translation.y > wrap.half_extents.y {
			transform.translation.y -= wrap.half_extents.y * 2.;
		} else if transform.translation.y < -wrap.half_extents.y {
			transform.translation.y += wrap.half_extents.y * 2.;
		}
		if transform.translation.z > wrap.half_extents.z {
			transform.translation.z -= wrap.half_extents.z * 2.;
		} else if transform.translation.z < -wrap.half_extents.z {
			transform.translation.z += wrap.half_extents.z * 2.;
		}
	}
}
