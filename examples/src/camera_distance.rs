use bevy::prelude::*;
use bevy::window::WindowResized;

/// Moves camera distance to keep the width in view on window resize
#[derive(Component)]
pub struct CameraDistance {
	pub width: f32,
	pub offset: Vec3,
}

impl Default for CameraDistance {
	fn default() -> Self {
		Self {
			width: 10.0,
			offset: Vec3::ZERO,
		}
	}
}

impl CameraDistance {
	pub fn new(x: f32) -> Self {
		Self {
			width: x,
			offset: Vec3::ZERO,
		}
	}
	pub fn new_with_origin(width: f32, origin: Vec3) -> Self {
		Self {
			width,
			offset: origin,
		}
	}
}


pub fn camera_distance(
	mut resize: EventReader<WindowResized>,
	mut query: Query<(&mut Transform, &Projection, &CameraDistance)>,
) {
	for e in resize.read() {
		// e.width;
		let aspect_ratio = e.width as f32 / e.height as f32;


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
}
