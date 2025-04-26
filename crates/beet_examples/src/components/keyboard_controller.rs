use bevy::prelude::*;
use sweet::prelude::When;

#[derive(Component, Reflect)]
#[reflect(Default, Component)]
pub struct KeyboardController {
	pub speed: f32,
}

impl Default for KeyboardController {
	fn default() -> Self { Self { speed: 3. } }
}

pub fn keyboard_controller(
	time: When<Res<Time>>,
	keys: When<Res<ButtonInput<KeyCode>>>,
	mut query: Populated<(&mut Transform, &KeyboardController)>,
) {
	for (mut transform, controller) in query.iter_mut() {
		let mut direction = Vec3::ZERO;
		if keys.pressed(KeyCode::KeyW) {
			direction -= Vec3::Z;
		}
		if keys.pressed(KeyCode::KeyS) {
			direction += Vec3::Z;
		}
		if keys.pressed(KeyCode::KeyA) {
			direction -= Vec3::X;
		}
		if keys.pressed(KeyCode::KeyD) {
			direction += Vec3::X;
		}
		if keys.pressed(KeyCode::KeyR) {
			direction += Vec3::Y;
		}
		if keys.pressed(KeyCode::KeyF) {
			direction -= Vec3::Y;
		}
		transform.translation +=
			direction * controller.speed * time.delta_secs();
	}
}
