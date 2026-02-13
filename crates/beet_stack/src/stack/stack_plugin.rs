use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct StackPlugin;


impl Plugin for StackPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<InterfacePlugin>()
			.init_plugin::<DocumentPlugin>()
			.init_plugin::<RouterPlugin>();
	}
}
