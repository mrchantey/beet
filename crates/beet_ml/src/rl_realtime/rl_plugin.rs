use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the [`SessionEntity`] component used to associate transient
/// entities (agents, episode-scoped scene nodes) with their owning RL
/// session.
#[derive(Default)]
pub struct RlPlugin;

impl Plugin for RlPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<SessionEntity>();
		app.world_mut().register_component::<SessionEntity>();
	}
}
