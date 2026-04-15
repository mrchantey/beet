//! Renderer-agnostic media rendering types.
//!
//! These types connect navigation sources (like [`Navigator`] in beet_router)
//! to render targets (like [`MediaParser`](crate::prelude::MediaParser)).

use beet_core::prelude::*;

/// Triggered on each [`RenderedBy`] entity when a navigator visits a new page.
#[derive(Debug, Clone, EntityEvent)]
pub struct RenderMedia {
	entity: Entity,
	media_bytes: MediaBytes,
}

impl RenderMedia {
	/// Create a new render media event for the given entity and media bytes.
	pub fn new(entity: Entity, media_bytes: MediaBytes) -> Self {
		Self {
			entity,
			media_bytes,
		}
	}
}

impl core::ops::Deref for RenderMedia {
	type Target = MediaBytes;
	fn deref(&self) -> &Self::Target { &self.media_bytes }
}

/// Assigned to a navigator, listing entities that should render on each
/// url visit.
#[derive(Debug, Clone, Component)]
#[relationship_target(relationship = RenderedBy)]
pub struct RenderTargets(Vec<Entity>);

/// Assigned to a render entity (eg `TuiNodeRenderer`) to make it a target
/// of a navigator's render calls.
#[derive(Debug, Clone, Component)]
#[relationship(relationship_target = RenderTargets)]
pub struct RenderedBy(pub Entity);
