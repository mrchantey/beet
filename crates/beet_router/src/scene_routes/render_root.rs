use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// On the rendered content entity: its [`RenderRoot`] handle (often itself).
///
/// The source side of the one-to-one [`RenderRoot`] relationship, the entity
/// whose tree the [`NodeRenderer`] walks, and the boundary at which in-tree
/// traversal stops (see [`RouteQuery`]). A fixed or per-request route is
/// self-referential — the content is its own handle — hence
/// `allow_self_referential`. An ephemeral coordinator route instead points a
/// persistent handle at a separately spawned content entity (see
/// [`RenderRoot::insert_rendered`]).
#[derive(Component)]
#[relationship(relationship_target = RenderRoot, allow_self_referential)]
pub struct RenderRootOf(pub Entity);

/// The handle of a render tree: names the content entity to walk and serialize.
///
/// The target side of the one-to-one relationship. [`RenderRoot::rendered`] is
/// the content entity (the [`RenderRootOf`] holder) the [`NodeRenderer`] walks;
/// it equals the handle itself for self-referential roots. The ephemeral
/// entities to clean up after render live separately on [`DespawnAfterRender`],
/// since cleanup is *not* derived from tree membership: a shared/cached fragment
/// can be slotted into a render without being owned by it.
#[derive(Component)]
#[relationship_target(relationship = RenderRootOf)]
pub struct RenderRoot {
	/// The content entity whose tree the [`NodeRenderer`] walks (the one-to-one
	/// source), equal to the handle for self-referential roots.
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
	///
	/// The common case: the entity is both handle and content. For an ephemeral
	/// coordinator that outlives the content it renders, see
	/// [`RenderRoot::insert_rendered`].
	pub fn insert(entity: &mut EntityWorldMut, to_despawn: Vec<Entity>) {
		let id = entity.id();
		entity.insert((RenderRootOf(id), DespawnAfterRender(to_despawn)));
	}

	/// Points render root `handle` at a separately spawned `rendered` content
	/// entity, recording the ephemerals cleaned up after render in `handle`'s
	/// [`DespawnAfterRender`].
	///
	/// The path for ephemeral coordinator routes, where a persistent handle (in
	/// the route tree) renders per-request content spawned elsewhere. The
	/// [`NodeRenderer`] walks `rendered`, not `handle`. For the common
	/// self-referential case see [`RenderRoot::insert`].
	pub fn insert_rendered(
		world: &mut World,
		handle: Entity,
		rendered: Entity,
		to_despawn: Vec<Entity>,
	) {
		world.entity_mut(rendered).insert(RenderRootOf(handle));
		world
			.entity_mut(handle)
			.insert(DespawnAfterRender(to_despawn));
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
			.with(|entity| -> Result<(Entity, Vec<Entity>)> {
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
			.with(move |world| {
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
		.with(move |mut entity| {
			RenderRoot::insert(&mut entity, default());
		})
		.await?;
	RenderRequest(id).xok()
}


/// Serves bytes from the ancestor [`BlobStore`] parsed into a render tree.
#[derive(Component, Reflect)]
#[require(BlobSceneAction)]
pub struct BlobScene {
	path: SmolPath,
}
impl BlobScene {
	pub fn new(path: impl Into<SmolPath>) -> Self { Self { path: path.into() } }
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
		.with(move |mut entity| -> Result {
			MediaParser::new().parse(ParseContext::new(&mut entity, &bytes))?;
			// derive per-page metadata from the parsed frontmatter, if any, so the
			// render context can expose this route's title/description/sidebar info.
			#[cfg(feature = "markdown_parser")]
			if let Some(meta) = entity
				.get::<beet_ui::prelude::Frontmatter>()
				.map(ArticleMeta::from_frontmatter)
			{
				entity.insert(meta);
			}
			RenderRoot::insert(&mut entity, default());
			Ok(())
		})
		.await??;
	RenderRequest(cx.id()).xok()
}
