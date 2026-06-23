//! The thread<->UI relationship: a [`ThreadView`](crate::prelude::ThreadView) or
//! [`ThreadComposer`](crate::prelude::ThreadComposer) points at the thread it
//! renders/drives via [`OfThread`], and the thread names all its UI items via
//! [`ThreadItems`]. One relationship carries both kinds; the projection + input
//! systems traverse `ThreadItems` (filtering by the item marker) rather than
//! scanning every view/composer for a stored entity.

use beet_core::prelude::*;

/// The thread a [`ThreadView`](crate::prelude::ThreadView) or
/// [`ThreadComposer`](crate::prelude::ThreadComposer) is bound to: the source half
/// of the [`ThreadItems`] relationship.
///
/// Spawn it directly beside the view/composer marker so the relationship machinery
/// remaps its `$thread` reference (deriving it from a still-placeholder field at
/// `on_add` would capture the placeholder):
/// `<div {(ThreadView, OfThread($thread))}/>`. `allow_self_referential` so an item
/// co-located with its `Thread` on one entity still links.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = ThreadItems, allow_self_referential)]
pub struct OfThread(#[entities] pub Entity);

impl OfThread {
	/// The thread entity this UI item is bound to.
	pub fn thread(&self) -> Entity { self.0 }
}

/// Every UI item (view, composer) bound to a thread: the target half of the
/// [`OfThread`] relationship, on the thread entity. The projection + input systems
/// traverse this, filtering by `With<ThreadView>` / `With<ThreadComposer>`, rather
/// than scanning by a stored entity.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = OfThread)]
pub struct ThreadItems(Vec<Entity>);
