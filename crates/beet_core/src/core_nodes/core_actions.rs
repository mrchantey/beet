use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
#[derive_action]
pub struct Translate {
	/// Translation to apply, in meters per second
	pub translation: Vec3,
}

impl Translate {
	pub fn new(translation: Vec3) -> Self { Self { translation } }
}

fn translate(
	mut _commands: Commands,
	time: Res<Time>,
	mut transforms: Query<&mut Transform>,
	query: Query<(&TargetAgent, &Translate), With<Running>>,
) {
	for (target, translate) in query.iter() {
		transforms.get_mut(**target).unwrap().translation +=
			translate.translation * time.delta_seconds();
	}
}
