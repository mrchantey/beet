//! [`ReflectDerived`] and [`Derived`]: the marks saying a type, or an entity
//! and its subtree, is derived state, never content.
use crate::prelude::*;
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

/// Marks an entity, and every descendant under it, as **derived state**: built
/// at runtime from content that is saved, so a dump skips the whole subtree.
///
/// The entity-level twin of [`ReflectDerived`]. A type is derived when no
/// authored content ever holds it; an entity is derived when something that
/// *is* content generated it, ie the widgets a scene editor spawns under the
/// tag that asked for them. The tag is saved and rebuilds its editor ui on the
/// next boot; the editor ui itself would only duplicate on reload.
///
/// Every saver honours it by construction: a subtree walk stops here, and an
/// entity carrying it, or under one that does, is never extracted.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct Derived;

impl Derived {
	/// Whether `entity` is derived: it carries the mark, or an owner does (its
	/// [`ChildOf`] parent, or the element an attribute entity belongs to).
	pub fn contains(world: &World, entity: Entity) -> bool {
		let mut current = Some(entity);
		while let Some(entity) = current {
			let Ok(entity) = world.get_entity(entity) else {
				return false;
			};
			if entity.contains::<Self>() {
				return true;
			}
			current = entity
				.get::<ChildOf>()
				.map(ChildOf::parent)
				.or_else(|| entity.get::<AttributeOf>().map(|attr| **attr));
		}
		false
	}
}
