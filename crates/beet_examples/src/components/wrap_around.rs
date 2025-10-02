use beet_core::prelude::*;

pub const DEFAULT_WRAPAROUND_HALF_EXTENTS: f32 = 1.;


#[derive(Debug, Clone, Resource, PartialEq, Reflect)]
#[reflect(Resource)]
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
	// pub fn cube(half_width: f32) -> Self {
	// 	Self {
	// 		half_extents: Vec3::new(half_width, half_width, half_width),
	// 	}
	// }
	pub fn from_window_size(val: Vec2) -> Self {
		Self {
			half_extents: Vec3::new(val.x, val.y, 0.1) * 0.5,
		}
	}
}

pub fn wrap_around(
	wrap: When<Res<WrapAround>>,
	mut query: Populated<&mut Transform, Changed<Transform>>,
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


// fn setup_wrap(mut commands: Commands, windows: Query<&Window>) {
// 	let size = windows.single().size();
// 	commands.insert_resource(WrapAround::from_window_size(size));
// }


pub fn update_wrap_around(
	mut wrap_around: When<ResMut<WrapAround>>,
	windows: Populated<&Window, Changed<Window>>,
) {
	for window in windows.iter() {
		wrap_around.set_if_neq(WrapAround::from_window_size(window.size()));
	}
}
