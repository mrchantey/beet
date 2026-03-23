use beet_core::prelude::*;

#[derive(Default)]
pub struct SocialPlugin {}

impl Plugin for SocialPlugin {
	fn build(&self, app: &mut App) { app.init_plugin::<AsyncPlugin>(); }
}
