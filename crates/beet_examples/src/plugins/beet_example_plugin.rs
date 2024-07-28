use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;

pub struct BeetExamplePlugin;

impl Plugin for BeetExamplePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((plugin_2d, plugin_3d))
			.register_type::<Collectable>();
	}
}

fn plugin_2d(app: &mut App) {
	app
		.add_systems(Update, follow_cursor_2d)
		// .add_systems(PreUpdate, auto_spawn::auto_spawn.before(PreTickSet))
		.add_systems(Update, randomize_position.in_set(PreTickSet))
		.add_systems(
			Update,
			(update_wrap_around, wrap_around)
			.chain()
			.run_if(|res: Option<Res<WrapAround>>| res.is_some())
			.in_set(PostTickSet),
		)
		.register_type::<AutoSpawn>()
		.register_type::<RandomizePosition>()
		.register_type::<WrapAround>()
		.register_type::<FollowCursor2d>()
		/*_*/;
}

fn plugin_3d(app: &mut App) {
	app.add_systems(
			Update,
			(follow_cursor_3d, camera_distance, rotate_collectables),
		)
		.register_type::<FollowCursor3d>()
		.register_type::<CameraDistance>()
		/*-*/;
}
