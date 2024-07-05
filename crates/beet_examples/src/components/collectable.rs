use bevy::prelude::*;



#[derive(Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
pub struct Collectable;




pub fn rotate_collectables(
	time: Res<Time>,
	mut query: Query<&mut Transform, With<Collectable>>,
) {
	for mut transform in query.iter_mut() {
		transform.rotate(Quat::from_rotation_y(time.delta_seconds() * 0.5));
	}
}
