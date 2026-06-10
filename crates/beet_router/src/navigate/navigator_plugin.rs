use crate::prelude::*;
use beet_core::prelude::*;


#[derive(Default)]
pub struct NavigatorPlugin;


impl Plugin for NavigatorPlugin {
	fn build(&self, app: &mut App) {
		// link click handling (internal nav vs external open) lives in
		// OpenLinkPlugin, which classifies a clicked `<a>` and routes it.
		app.init_plugin::<OpenLinkPlugin>()
			.add_observer(single_current_scene);
	}
}
