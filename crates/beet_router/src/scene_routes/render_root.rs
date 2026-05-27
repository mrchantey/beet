use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// On the rendered content entity → its render root (often itself).
///
/// The source side of the one-to-one [`RenderRoot`] relationship. A fixed or
/// per-request route points only its rendered entity at the root, so the
/// relationship stays singular; the root is usually its own rendered entity,
/// hence `allow_self_referential`.
#[derive(Component)]
#[relationship(relationship_target = RenderRoot, allow_self_referential)]
pub struct RenderRootOf(pub Entity);

/// On the render root: the entity whose tree the [`NodeRenderer`] walks.
///
/// Marks the boundary of a render tree (where in-tree traversal stops) and
/// names the entity to serialize. The ephemeral entities to clean up after
/// render live separately on [`DespawnAfterRender`], since cleanup is *not*
/// derived from tree membership: a shared/cached fragment can be slotted into a
/// render without being owned by it.
#[derive(Component)]
#[relationship_target(relationship = RenderRootOf)]
pub struct RenderRoot {
	/// The entity whose tree the [`NodeRenderer`] walks (the one-to-one source).
	#[relationship]
	rendered: Entity,
}

/// Ephemeral entities despawned once their render root has been rendered, ie a
/// per-request page or a help/not-found tree.
#[derive(Default, Component)]
pub struct DespawnAfterRender(pub Vec<Entity>);

impl RenderRoot {
	/// The entity whose tree the [`NodeRenderer`] walks.
	pub fn rendered(&self) -> Entity { self.rendered }

	/// Marks `entity` as a self-referential render root, recording the
	/// ephemeral entities cleaned up after render in [`DespawnAfterRender`].
	pub fn insert(entity: &mut EntityWorldMut, to_despawn: Vec<Entity>) {
		let id = entity.id();
		entity.insert((RenderRootOf(id), DespawnAfterRender(to_despawn)));
	}

	/// Renders a render-root entity through the full pipeline:
	/// 1. ancestor render middleware (layout wrapping, etc.) — `Entity` → `Entity`
	/// 2. the [`RequestParts`]/[`Response`] renderer middleware
	///
	/// Reads [`RenderRoot::rendered`] off the (post-middleware) root to find
	/// what to serialize, then despawns the [`DespawnAfterRender`] entities.
	pub async fn render(
		root: Entity,
		caller: &AsyncEntity,
		parts: RequestParts,
	) -> Result<Response> {
		// apply ancestor render middleware (layout wrapping, etc.)
		let root = caller
			.call_with_middleware(Action::new_fixed(root), parts.clone())
			.await?;

		let render_action = caller
			.with_state::<MiddlewareQuery<RequestParts, Response>, _>(
				move |entity, query| {
					query.resolve_action(
						entity,
						Action::new_async(default_renderer),
					)
				},
			)
			.await?;

		// read what to render and what to clean up off the render root
		let (rendered, to_despawn) = caller
			.world()
			.entity(root)
			.with_then(|entity| -> Result<(Entity, Vec<Entity>)> {
				let rendered = entity
					.get::<RenderRoot>()
					.ok_or_else(|| {
						bevyhow!("entity {} is not a render root", entity.id())
					})?
					.rendered;
				let to_despawn = entity
					.get::<DespawnAfterRender>()
					.map(|despawn| despawn.0.clone())
					.unwrap_or_default();
				(rendered, to_despawn).xok()
			})
			.await??;

		let result = caller
			.world()
			.entity(rendered)
			.call_detached(render_action, parts)
			.await;

		// despawn all ephemeral entities
		caller
			.world()
			.with_then(move |world| {
				for entity in to_despawn {
					if let Ok(entity) = world.get_entity_mut(entity) {
						entity.despawn();
					}
				}
			})
			.await;

		result
	}
}

/// The output handle of a scene route: a newtype over the render-root
/// [`Entity`].
///
/// A dedicated type (rather than a bare `Entity`) is required so the
/// [`ExchangeRouteOut`] impl does not collide with the blanket `Serialize`
/// impl — `Entity` is itself `Serialize`. The despawn list still lives on the
/// entity's [`DespawnAfterRender`], not on this handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderRequest(pub Entity);

impl ExchangeRouteOut<Self> for RenderRequest {
	fn into_route_response(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move { RenderRoot::render(self.0, &caller, parts).await })
	}
}

/// Returns the caller as a self-referential render root.
#[action(route)]
#[derive(Default, Component)]
pub(crate) async fn CallerScene(
	cx: ActionContext<Request>,
) -> Result<RenderRequest> {
	let id = cx.id();
	cx.caller
		.with_then(move |mut entity| {
			RenderRoot::insert(&mut entity, default());
		})
		.await?;
	RenderRequest(id).xok()
}


/// Serves bytes from the ancestor [`BlobStore`] parsed into a render tree.
#[derive(Component, Reflect)]
#[require(BlobSceneAction)]
pub struct BlobScene {
	path: RelPath,
}
impl BlobScene {
	pub fn new(path: impl Into<RelPath>) -> Self { Self { path: path.into() } }
}


#[action(route)]
#[derive(Default, Component)]
async fn BlobSceneAction(cx: ActionContext<Request>) -> Result<RenderRequest> {
	let store = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, BlobStore>(|entity, query| {
			query
				.get(entity)
				.cloned()
				.unwrap_or_else(|_| BlobStore::new(FsStore::default()))
		})
		.await?;

	let path = cx.caller.get::<BlobScene, _>(|fs| fs.path.clone()).await?;
	let bytes = store.get_media(&path).await?;

	cx.caller
		.with_then(move |mut entity| -> Result {
			MediaParser::new().parse(ParseContext::new(&mut entity, &bytes))?;
			RenderRoot::insert(&mut entity, default());
			Ok(())
		})
		.await??;
	RenderRequest(cx.id()).xok()
}
