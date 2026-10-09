//! The render root: the entity a render walks, and the response a built page
//! answers a request with.
//!
//! [`PageRoot`] names the entity the [`NodeRenderer`] walks, and a scene
//! route's [`PageRequest`] answers through [`LivePage::respond`], which
//! prepares, renders and releases it.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// On the rendered content entity: its [`PageRoot`] handle (often itself).
///
/// The source side of the one-to-one [`PageRoot`] relationship, the entity
/// whose tree the [`NodeRenderer`] walks, and the boundary at which in-tree
/// traversal stops (see [`RouteQuery`]). A fixed or per-request route is
/// self-referential, the content being its own handle, hence
/// `allow_self_referential`. An ephemeral coordinator route instead points a
/// persistent handle at a separately spawned content entity (see
/// [`PageRoot::insert_rendered`]).
#[derive(Component)]
#[relationship(relationship_target = PageRoot, allow_self_referential)]
pub struct PageRootOf(pub Entity);

/// The handle of a render tree: names the content entity to walk and serialize.
///
/// The target side of the one-to-one relationship. [`PageRoot::rendered`] is
/// the content entity (the [`PageRootOf`] holder) the [`NodeRenderer`] walks;
/// it equals the handle itself for self-referential roots. The ephemeral
/// entities to clean up after render live separately on [`DespawnAfterRender`],
/// since cleanup is *not* derived from tree membership: a shared fragment
/// can be slotted into a render without being owned by it.
#[derive(Component)]
#[relationship_target(relationship = PageRootOf)]
pub struct PageRoot {
	/// The content entity whose tree the [`NodeRenderer`] walks (the one-to-one
	/// source), equal to the handle for self-referential roots.
	#[relationship]
	rendered: Entity,
}

/// Ephemeral entities despawned once their render root has been rendered, ie a
/// per-request page or a help/not-found tree.
#[derive(Default, Component)]
pub struct DespawnAfterRender(pub Vec<Entity>);

impl PageRoot {
	/// The entity whose tree the [`NodeRenderer`] walks.
	pub fn rendered(&self) -> Entity { self.rendered }

	/// Marks `entity` as a self-referential render root, recording the
	/// ephemeral entities cleaned up after render in [`DespawnAfterRender`].
	///
	/// The common case: the entity is both handle and content. For an ephemeral
	/// coordinator that outlives the content it renders, see
	/// [`PageRoot::insert_rendered`].
	pub fn insert(entity: &mut EntityWorldMut, to_despawn: Vec<Entity>) {
		let id = entity.id();
		entity.insert((PageRootOf(id), DespawnAfterRender(to_despawn)));
	}

	/// Points render root `handle` at a separately spawned `rendered` content
	/// entity, recording the ephemerals cleaned up after render in `handle`'s
	/// [`DespawnAfterRender`].
	///
	/// The path for ephemeral coordinator routes, where a persistent handle (in
	/// the route tree) renders per-request content spawned elsewhere. The
	/// [`NodeRenderer`] walks `rendered`, not `handle`. For the common
	/// self-referential case see [`PageRoot::insert`].
	pub fn insert_rendered(
		world: &mut World,
		handle: Entity,
		rendered: Entity,
		to_despawn: Vec<Entity>,
	) {
		world.entity_mut(rendered).insert(PageRootOf(handle));
		world
			.entity_mut(handle)
			.insert(DespawnAfterRender(to_despawn));
	}
}

impl IntoResponseWithRequestParts<Self> for PageRequest {
	fn into_response_with_request_parts(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move { LivePage::respond(self.0, &caller, parts).await })
	}
}
