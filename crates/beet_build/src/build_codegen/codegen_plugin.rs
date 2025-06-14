use crate::prelude::*;
use bevy::prelude::*;


pub struct CodegenPlugin;


impl Plugin for CodegenPlugin {
	fn build(&self, app: &mut App) {
		app.init_non_send_resource::<CodegenConfig>().add_systems(
			Update,
			(
				spawn_route_files,
				parse_route_file_rust,
				// (parse_route_file_rust, parse_route_file_markdown),
				modify_file_route_tokens,
			)
				.chain(),
		);
	}
}
