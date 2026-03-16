use crate::types::ContextMap;
use beet_core::prelude::*;

#[derive(Default)]
pub struct ClankerPlugin {}

impl Plugin for ClankerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_resource::<ContextMap>();
	}
}
