use bevy::prelude::*;

#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct FollowCursor2d;

pub fn follow_cursor_2d(
	camera_query: Query<(&Camera, &GlobalTransform)>,
	mut cursor_query: Query<&mut Transform, With<FollowCursor2d>>,
	windows: Query<&Window>,
) {
	let Ok((camera, camera_transform)) = camera_query.get_single() else {
		return;
	};

	let Some(cursor_position) = windows
		.get_single()
		.ok()
		.map(|w| w.cursor_position())
		.flatten()
	else {
		return;
	};

	let Ok(point) =
		camera.viewport_to_world_2d(camera_transform, cursor_position)
	else {
		return;
	};

	for mut transform in cursor_query.iter_mut() {
		transform.translation = point.extend(0.);
	}
}


#[derive(Component, Reflect)]
#[reflect(Default, Component)]
pub struct FollowCursor3d {
	pub intersect_point: Vec3,
	pub intersect_plane: InfinitePlane3d,
}

impl FollowCursor3d {
	/// Follows the cursor on the XZ plane with the Y axis as the normal.
	pub const ORIGIN_Y: Self = Self {
		intersect_point: Vec3::ZERO,
		intersect_plane: InfinitePlane3d { normal: Dir3::Y },
	};

	/// Follows the cursor on the XY plane with the Z axis as the normal.
	pub const ORIGIN_Z: Self = Self {
		intersect_point: Vec3::ZERO,
		intersect_plane: InfinitePlane3d { normal: Dir3::Z },
	};


	pub fn new(intersect_point: Vec3, intersect_plane: Vec3) -> Self {
		Self {
			intersect_point,
			intersect_plane: InfinitePlane3d::new(intersect_plane),
		}
	}
	pub fn with_intersect_point(self, intersect_point: Vec3) -> Self {
		Self {
			intersect_point,
			..self
		}
	}
	pub fn with_intersect_plane(self, intersect_plane: Vec3) -> Self {
		Self {
			intersect_plane: InfinitePlane3d::new(intersect_plane),
			..self
		}
	}
}

impl Default for FollowCursor3d {
	fn default() -> Self { Self::ORIGIN_Y }
}

pub fn follow_cursor_3d(
	camera_query: Query<(&Camera, &GlobalTransform)>,
	mut cursor_query: Query<(&mut Transform, &FollowCursor3d)>,
	windows: Query<&Window>,
) {
	let Ok((camera, camera_transform)) = camera_query.get_single() else {
		return;
	};

	let Some(cursor_position) = windows
		.get_single()
		.ok()
		.map(|w| w.cursor_position())
		.flatten()
	else {
		return;
	};
	let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position)
	else {
		return;
	};

	for (mut transform, follow_cursor) in cursor_query.iter_mut() {
		let Some(dist) = ray.intersect_plane(
			follow_cursor.intersect_point,
			follow_cursor.intersect_plane,
		) else {
			continue;
		};
		let point = ray.get_point(dist);
		transform.translation = point;
	}
}
