use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct ToolPlugin;


impl Plugin for ToolPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(insert_tool_path_and_params);
		app.add_observer(insert_tool_tree);
	}
}
