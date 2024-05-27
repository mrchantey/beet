use bevy::prelude::*;
use bevy::window::WindowResized;



/// Given a value  `x`, move camera backward to include that value
#[derive(Component)]
pub struct CameraDistance {
	pub x: f32,
	pub origin: Vec3,
}

impl Default for CameraDistance {
	fn default() -> Self {
		Self {
			x: 10.0,
			origin: Vec3::ZERO,
		}
	}
}

impl CameraDistance {
	pub fn new(x: f32) -> Self {
		Self {
			x,
			origin: Vec3::ZERO,
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
			let z = camera_distance.x / f32::tan(hfov * 0.5);

			// log::info!("z: {}", z);

			// let z = camera_distance.x / fov.tan();


			let fwd = transform.forward();
			let pos = camera_distance.origin - fwd * z;
			transform.translation = pos;
		}
	}
}
