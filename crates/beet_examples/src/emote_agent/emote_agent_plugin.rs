use crate::prelude::*;
use bevy::prelude::*;



pub fn emote_agent_plugin(app: &mut App) {
	app.add_observer(apply_render_layers_to_children)
		.add_systems(
			Update,
			(
				ik_spawner.never_param_warn(),
				update_emoji_swapper.never_param_warn(),
			),
		)
		.register_type::<IkSpawner>();
}
