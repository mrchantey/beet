use bevy::prelude::*;

#[derive(Component)]
pub struct FollowCursor;

pub fn follow_cursor(
	camera_query: Query<(&Camera, &GlobalTransform)>,
	mut cursor_query: Query<&mut Transform, With<FollowCursor>>,
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
