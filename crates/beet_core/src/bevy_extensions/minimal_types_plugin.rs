//! Plugin registering the hierarchy relationship types required for world
//! serialization to round-trip parent/child links.

use crate::prelude::*;

/// Registers the minimal Bevy hierarchy types ([`ChildOf`], [`Children`])
/// so world serde can serialize parent/child relationships without each
/// downstream plugin re-registering them.
///
/// Use [`App::init_plugin::<MinimalTypesPlugin>`](BeetCoreAppExt::init_plugin)
/// to attach idempotently.
#[derive(Default)]
pub struct MinimalTypesPlugin;

impl Plugin for MinimalTypesPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<ChildOf>().register_type::<Children>();
	}
}
