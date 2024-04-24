use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use forky_bevy::extensions::AppExt;

/// Required Resources:
/// - [`Time`]
#[derive(Default)]
pub struct SteerPlugin {
	pub wrap_around: Option<WrapAround>,
}


impl Plugin for SteerPlugin {
	fn build(&self, app: &mut App) {
		app.__()
			.add_systems(
				Update,
				(
					integrate_force,
					wrap_around
						.run_if(|res: Option<Res<WrapAround>>| res.is_some()),
				)
					.chain()
					.in_set(PostTickSet),
			)
			.__();
		if let Some(wrap_around) = self.wrap_around.as_ref() {
			app.insert_resource(wrap_around.clone());
		}
	}
}
