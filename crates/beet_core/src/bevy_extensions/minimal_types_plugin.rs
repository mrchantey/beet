//! Plugin registering the fundamental Bevy identity and hierarchy types so
//! world serde round-trips them and BSX resolves them as tags/spreads.

use crate::prelude::*;

/// Registers the minimal Bevy types every app shares: [`Name`] (so `<Name("x")>`
/// resolves as a BSX tag and serde round-trips entity names) and the hierarchy
/// relationship ([`ChildOf`], [`Children`], so parent/child links survive a
/// round-trip) plus the membership one ([`Group`], [`MemberOf`], [`Members`]),
/// without each downstream plugin re-registering them.
///
/// It also marks the bevy clocks [`Derived`](ReflectDerived), the one place a
/// foreign type can carry the mark, so no dump site has to remember to deny
/// them.
///
/// Use [`App::add_type::<MinimalTypesPlugin>`](AppPluginVariantsExt::add_type)
/// to attach idempotently.
#[derive(Default)]
pub struct MinimalTypesPlugin;

impl Plugin for MinimalTypesPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Name>()
			.register_type::<ChildOf>()
			.register_type::<Children>()
			// ordered membership, the hierarchy's sibling axis
			.register_type::<Group>()
			.register_type::<MemberOf>()
			.register_type::<Members>()
			// the clocks advance every frame and are never authored content, so
			// they are marked once here rather than denied at each dump site.
			.register_type::<Time>()
			.register_type_data::<Time, ReflectDerived>()
			.register_type::<Time<Real>>()
			.register_type_data::<Time<Real>, ReflectDerived>()
			.register_type::<Time<Virtual>>()
			.register_type_data::<Time<Virtual>, ReflectDerived>()
			.register_type::<Time<Fixed>>()
			.register_type_data::<Time<Fixed>, ReflectDerived>();
	}
}
