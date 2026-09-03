//! [`ReflectDerived`]: the mark saying a type is derived state, never content.
use bevy_reflect::FromType;

/// Reflect type data marking a type as **derived state**: present in a running
/// world but never part of a scene's authored content, so a dump skips it.
///
/// Registration is already the free baseline: a type nothing opted into the
/// registry never dumps. This mark is for the types that *are* registered for
/// other reasons and still must not be saved, ie a frame clock
/// ([`Time`](bevy::prelude::Time)) or a reactively recomputed path
/// (`ResolvedFieldPath`). It is declared once, where the type is:
///
/// ```
/// # use beet_core::prelude::*;
/// #[derive(Component, Reflect)]
/// #[reflect(Component, Derived)]
/// struct CachedExtents(f32);
/// ```
///
/// A foreign type is marked at registration instead, with
/// [`App::register_derived`](crate::prelude::BeetCoreAppExt::register_derived).
///
/// Because the mark travels with the type, there are no per-dump-site deny
/// lists: every saver skips every derived type by construction, so a new dump
/// site cannot forget one and a new derived type cannot leak into an old site.
#[derive(Debug, Copy, Clone)]
pub struct ReflectDerived;

impl<T> FromType<T> for ReflectDerived {
	fn from_type() -> Self { Self }
}
