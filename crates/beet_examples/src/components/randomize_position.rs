use bevy::prelude::*;
use sweet::prelude::*;

#[derive(Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RandomizePosition {
	pub offset: Vec3,
	pub scale: Vec3,
}
impl Default for RandomizePosition {
	fn default() -> Self {
		Self {
			offset: Vec3::ZERO,
			scale: Vec3::ONE,
		}
	}
}


pub fn randomize_position(
	mut commands: Commands,
	mut rand: ResMut<RandomSource>,
	mut query: Populated<
		(Entity, &mut Transform, &RandomizePosition),
		Added<RandomizePosition>,
	>,
) {
	for (entity, mut transform, pos) in query.iter_mut() {
		let mut position = Vec3::random_in_cube(&mut rand);
		position.x *= pos.scale.x;
		position.y *= pos.scale.y;
		position.z *= pos.scale.z;
		transform.translation = pos.offset + position;
		commands.entity(entity).remove::<RandomizePosition>();
	}
}
