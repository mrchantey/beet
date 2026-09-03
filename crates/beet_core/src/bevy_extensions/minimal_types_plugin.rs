//! Plugin registering the fundamental Bevy identity and hierarchy types so
//! world serde round-trips them and BSX resolves them as tags/spreads.

use crate::prelude::*;

/// Registers the minimal Bevy types every app shares: [`Name`] (so `<Name("x")>`
/// resolves as a BSX tag and serde round-trips entity names) and the hierarchy
/// relationship ([`ChildOf`], [`Children`], so parent/child links survive a
/// round-trip), without each downstream plugin re-registering them.
///
/// It also marks the bevy clocks [`Derived`](ReflectDerived), the one place a
/// foreign type can carry the mark, so no dump site has to remember to deny
/// them.
///
/// Use [`App::init_plugin::<MinimalTypesPlugin>`](BeetCoreAppExt::init_plugin)
/// to attach idempotently.
#[derive(Default)]
pub struct MinimalTypesPlugin;

impl Plugin for MinimalTypesPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Name>()
			.register_type::<ChildOf>()
			.register_type::<Children>()
			// the clocks advance every frame and are never authored content, so
			// they are marked once here rather than denied at each dump site.
			.register_derived::<Time>()
			.register_derived::<Time<Real>>()
			.register_derived::<Time<Virtual>>()
			.register_derived::<Time<Fixed>>();
	}
}
