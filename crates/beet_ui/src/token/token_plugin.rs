use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct TokenPlugin;


impl Plugin for TokenPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<RuleSet>();
	}
}
