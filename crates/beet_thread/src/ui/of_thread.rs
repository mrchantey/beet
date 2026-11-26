//! The thread<->UI relationship: a [`ThreadView`](crate::prelude::ThreadView) or
//! [`CreatePostForm`](crate::prelude::CreatePostForm) points at the window,
//! a thread entity, it renders or posts to via [`OfThread`], and the window names
//! all its UI items via [`ThreadItems`]. One relationship carries both kinds,
//! so a window lists its views and composers without a scan.

use beet_core::prelude::*;

/// The window a [`ThreadView`](crate::prelude::ThreadView) or
/// [`CreatePostForm`](crate::prelude::CreatePostForm) is bound to: the source half
/// of the [`ThreadItems`] relationship.
///
/// Spawn it directly beside the view/composer marker so the relationship machinery
/// remaps its `$thread` reference (deriving it from a still-placeholder field at
/// `on_add` would capture the placeholder):
/// `<ThreadView {OfThread($thread)}/>`. `allow_self_referential` so an item
/// co-located with its `Thread` on one entity still links.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = ThreadItems, allow_self_referential)]
pub struct OfThread(#[entities] pub Entity);

impl OfThread {
	/// The window this UI item is bound to.
	pub fn thread(&self) -> Entity { self.0 }
}

/// Every UI item (view, form) bound to a window: the target half of the
/// [`OfThread`] relationship, on the window, so the window names each item
/// bound to it, a view or a composer by its marker.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = OfThread)]
pub struct ThreadItems(Vec<Entity>);
