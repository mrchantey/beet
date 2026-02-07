use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct StackPlugin;


impl Plugin for StackPlugin {
	fn build(&self, app: &mut App) { app.init_plugin::<DocumentPlugin>(); }
}
