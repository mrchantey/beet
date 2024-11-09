use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::window::WindowResized;

/// Moves camera distance to keep the width in view on window resize
#[derive(Component, Reflect)]
#[reflect(Default, Component)]
pub struct CameraDistance {
	pub width: f32,
	pub offset: Vec3,
}

impl Default for CameraDistance {
	fn default() -> Self { Self::new(10.) }
}

impl CameraDistance {
	pub fn new(scale: f32) -> Self {
		Self {
			width: scale * 1.1,
			offset: Vec3::new(0., scale, scale),
		}
	}
}


pub fn camera_distance(
	mut resize: EventReader<WindowResized>,
	main_window: Single<&Window, With<PrimaryWindow>>,
	camera_added: Query<
		(),
		(
			With<CameraDistance>,
			Or<(Added<Camera>, Added<CameraDistance>)>,
		),
	>,
	mut query: Populated<(&mut Transform, &Projection, &CameraDistance)>,
) {
	if camera_added.iter().count() == 0 && resize.read().count() == 0 {
		return;
	}

	let aspect_ratio = main_window.width() as f32 / main_window.height() as f32;

	for (mut transform, projection, camera_distance) in query.iter_mut() {
		let Projection::Perspective(perspective) = projection else {
			continue;
		};
		let fov = perspective.fov * 0.5;

		let hfov = 2. * f32::atan(f32::tan(fov) * aspect_ratio);
		let z = camera_distance.width / f32::tan(hfov * 0.5);

		// log::info!("z: {}", z);
		// let z = camera_distance.x / fov.tan();
		// let fwd = transform.forward();
		let fwd = camera_distance.offset.normalize();
		let pos = camera_distance.offset + fwd * z;
		transform.translation = pos;
		transform.look_at(Vec3::ZERO, Vec3::Y);
	}
}
