use bevy::prelude::*;

#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct FollowCursor2d;

pub fn follow_cursor_2d(
	camera_query: Query<(&Camera, &GlobalTransform)>,
	mut cursor_query: Query<&mut Transform, With<FollowCursor2d>>,
	windows: Query<&Window>,
) {
	let (camera, camera_transform) = camera_query.single();

	let Some(cursor_position) = windows.single().cursor_position() else {
		return;
	};

	let Some(point) =
		camera.viewport_to_world_2d(camera_transform, cursor_position)
	else {
		return;
	};

	for mut transform in cursor_query.iter_mut() {
		transform.translation = point.extend(0.);
	}
}


#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct FollowCursor3d;

pub fn follow_cursor_3d(
	camera_query: Query<(&Camera, &GlobalTransform)>,
	mut cursor_query: Query<&mut Transform, With<FollowCursor3d>>,
	windows: Query<&Window>,
) {
	let Ok((camera, camera_transform)) = camera_query.get_single() else {
		return;
	};

	let Some(cursor_position) = windows.single().cursor_position() else {
		return;
	};

	let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position)
	else {
		return;
	};

	let Some(dist) =
		ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
	else {
		return;
	};
	let point = ray.get_point(dist);

	for mut transform in cursor_query.iter_mut() {
		transform.translation = point;
	}
}
