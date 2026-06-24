use beet_core::prelude::*;

pub const DEFAULT_WRAPAROUND_HALF_EXTENTS: f32 = 1.;

/// Marker for an entity that wraps around the screen edges, the 2d "asteroids"
/// wrap. Opt-in: without it an entity is never wrapped, so 3d agents (which move
/// on all three axes) are unaffected by this 2d-only mechanic.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct WrapAround;

/// The half-extents of the 2d screen a [`WrapAround`] entity wraps within.
#[derive(Debug, Clone, Resource, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct WrapAroundBounds {
	pub half_extents: Vec2,
}

impl Default for WrapAroundBounds {
	fn default() -> Self {
		Self {
			half_extents: Vec2::splat(DEFAULT_WRAPAROUND_HALF_EXTENTS),
		}
	}
}

impl WrapAroundBounds {
	pub fn from_window_size(size: Vec2) -> Self {
		Self {
			half_extents: size * 0.5,
		}
	}
}

/// Wraps every [`WrapAround`] entity around the [`WrapAroundBounds`] on the XY
/// plane, eg a boid leaving the right edge reappears on the left. Only X and Y
/// wrap: Z is the 2d depth axis (and the 3d ground axis), so wrapping it would
/// pin agents to the Z origin.
pub fn wrap_around(
	bounds: When<Res<WrapAroundBounds>>,
	mut query: Populated<
		&mut Transform,
		(Changed<Transform>, With<WrapAround>),
	>,
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

pub fn update_wrap_around(
	mut bounds: When<ResMut<WrapAroundBounds>>,
	windows: Populated<&Window, Changed<Window>>,
) {
	for window in windows.iter() {
		bounds.set_if_neq(WrapAroundBounds::from_window_size(window.size()));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn wraps_opted_in_xy_only() {
		let mut app = App::new();
		app.insert_resource(WrapAroundBounds {
			half_extents: Vec2::splat(10.),
		})
		.add_systems(Update, wrap_around);

		// past the +x edge and well past where a z-wrap would trigger.
		let wrapped = app
			.world_mut()
			.spawn((Transform::from_xyz(12., 0., 50.), WrapAround))
			.id();
		// no marker: never wrapped, even though it is past the edge.
		let unmarked = app
			.world_mut()
			.spawn(Transform::from_xyz(12., 0., 50.))
			.id();

		app.update();

		// x wraps (12 - 20 = -8); z is left untouched (the seek_3d regression).
		app.world()
			.get::<Transform>(wrapped)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(-8., 0., 50.));
		app.world()
			.get::<Transform>(unmarked)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(12., 0., 50.));
	}
}
