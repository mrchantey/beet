//! Plugin registering the fundamental Bevy identity and hierarchy types so
//! world serde round-trips them and BSX resolves them as tags/spreads.

use crate::prelude::*;

/// Registers the minimal Bevy types every app shares: [`Name`] (so `<Name("x")>`
/// resolves as a BSX tag and serde round-trips entity names) and the hierarchy
/// relationship ([`ChildOf`], [`Children`], so parent/child links survive a
/// round-trip), without each downstream plugin re-registering them.
///
/// Use [`App::init_plugin::<MinimalTypesPlugin>`](BeetCoreAppExt::init_plugin)
/// to attach idempotently.
#[derive(Default)]
pub struct MinimalTypesPlugin;

impl Plugin for MinimalTypesPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Name>()
			.register_type::<ChildOf>()
			.register_type::<Children>();
	}
}
