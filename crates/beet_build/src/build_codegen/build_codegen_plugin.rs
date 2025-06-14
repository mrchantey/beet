use bevy::prelude::*;

use crate::prelude::CodegenConfig;



pub struct BuildCodegenPlugin;


impl Plugin for BuildCodegenPlugin {
	fn build(&self, app: &mut App) {
		app.init_non_send_resource::<CodegenConfig>();
	}
}
