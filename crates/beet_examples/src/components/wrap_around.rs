use beet_core::prelude::*;

pub const DEFAULT_WRAPAROUND_HALF_EXTENTS: f32 = 1.;

/// Marker for an entity that wraps around the screen edges, the 2d "asteroids"
/// wrap. Opt-in: without it an entity is never wrapped, so 3d agents (which move
/// on all three axes) are unaffected by this 2d-only mechanic.
#[derive(Debug, Default, Clone, Copy, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
pub struct WrapAround;

/// The half-extents a [`WrapAround`] entity wraps within.
#[derive(Debug, Clone, Resource, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct WrapAroundBounds {
	pub half_extents: Vec3,
}

impl Default for WrapAroundBounds {
	fn default() -> Self {
		Self {
			half_extents: Vec3::splat(DEFAULT_WRAPAROUND_HALF_EXTENTS),
		}
	}
}

impl WrapAroundBounds {
	pub fn from_window_size(size: Vec2) -> Self {
		Self {
			half_extents: Vec3::new(size.x, size.y, 0.1) * 0.5,
		}
	}
}

/// Wraps every [`WrapAround`] entity around the [`WrapAroundBounds`], eg a boid
/// leaving the right edge reappears on the left. Opt-in via the [`WrapAround`]
/// marker, so an agent that should move freely (eg a 3d steering agent on the
/// ground plane) is never wrapped.
pub fn wrap_around(
	bounds: When<Res<WrapAroundBounds>>,
	mut query: Populated<
		&mut Transform,
		(Changed<Transform>, With<WrapAround>),
	>,
) {
	for mut transform in query.iter_mut() {
		if transform.translation.x > bounds.half_extents.x {
			transform.translation.x -= bounds.half_extents.x * 2.;
		} else if transform.translation.x < -bounds.half_extents.x {
			transform.translation.x += bounds.half_extents.x * 2.;
		}
		if transform.translation.y > bounds.half_extents.y {
			transform.translation.y -= bounds.half_extents.y * 2.;
		} else if transform.translation.y < -bounds.half_extents.y {
			transform.translation.y += bounds.half_extents.y * 2.;
		}
		if transform.translation.z > bounds.half_extents.z {
			transform.translation.z -= bounds.half_extents.z * 2.;
		} else if transform.translation.z < -bounds.half_extents.z {
			transform.translation.z += bounds.half_extents.z * 2.;
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
	fn wraps_only_opted_in() {
		let mut app = App::new();
		app.insert_resource(WrapAroundBounds {
			half_extents: Vec3::splat(10.),
		})
		.add_systems(Update, wrap_around);

		// opted-in: wraps on every axis past the bound.
		let wrapped = app
			.world_mut()
			.spawn((Transform::from_xyz(12., 0., 12.), WrapAround))
			.id();
		// no marker: never wrapped, so a 3d agent moving freely (eg the seek_3d
		// fox on the ground plane) is unaffected.
		let unmarked = app
			.world_mut()
			.spawn(Transform::from_xyz(12., 0., 12.))
			.id();

		app.update();

		// x and z both wrap (12 - 2*10 = -8); the marker gates it.
		app.world()
			.get::<Transform>(wrapped)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(-8., 0., -8.));
		app.world()
			.get::<Transform>(unmarked)
			.unwrap()
			.translation
			.xpect_eq(Vec3::new(12., 0., 12.));
	}
}
