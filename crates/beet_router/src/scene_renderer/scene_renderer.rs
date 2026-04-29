use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;

/// A route output representing a scene entity to be rendered.
/// Entities in `despawn` are cleaned up after rendering,
/// ie help pages, not-found pages.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SceneEntity {
	/// The entity to render.
	pub entity: Entity,
	/// Entities to despawn after rendering.
	pub despawn: Vec<Entity>,
}
impl SceneEntity {
	/// A scene entity that should not be despawned after render.
	pub fn new_fixed(entity: Entity) -> Self {
		Self {
			entity,
			despawn: default(),
		}
	}

	/// A scene entity that should be despawned after render,
	/// ie a help page or not found route.
	pub fn new_ephemeral(entity: Entity) -> Self {
		Self {
			entity,
			despawn: vec![entity],
		}
	}
	pub fn push_despawn(mut self, entity: Entity) -> Self {
		self.despawn.push(entity);
		self
	}

	/// Merge another scene's despawn list into this one.
	pub fn with_join(mut self, child: SceneEntity) -> Self {
		self.despawn.extend(child.despawn);
		self
	}
	pub async fn render(
		self,
		caller: &AsyncEntity,
		parts: RequestParts,
	) -> Result<Response> {
		// apply ancestor scene middleware (layout wrapping, etc.)
		let scene = caller
			.call_with_middleware(Action::new_fixed(self), parts.clone())
			.await?;

		let render_action = caller
			.with_state::<MiddlewareQuery<RequestParts, Response>, _>(
				move |entity, query| {
					query.resolve_action(
						entity,
						Action::new_async(default_scene_renderer),
					)
				},
			)
			.await;

		let scene_entity = caller.world().entity(scene.entity);
		let result = scene_entity.call_detached(render_action, parts).await;

		// despawn all ephemeral entities
		caller
			.world()
			.with_then(|world| {
				for entity in scene.despawn {
					world.entity_mut(entity).despawn();
				}
			})
			.await;

		result
	}
}

impl ExchangeRouteOut<Self> for SceneEntity {
	fn into_route_response(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move { self.render(&caller, parts).await })
	}
}

/// Creates a fixed routable scene from a path and content bundle.
///
/// The entity itself becomes both the route and the scene content.
/// The [`ExchangeAction`] handles the `Request` → `Response`
/// conversion.
///
/// # Example
///
/// ```no_run
/// use beet_router::prelude::*;
/// use beet_core::prelude::*;
/// use beet_node::prelude::*;
///
/// let bundle = fixed_scene("about",
///     Element::new("p").with_inner_text("About page")
/// );
/// ```
pub fn fixed_scene<B: Bundle>(path: &str, bundle: B) -> impl Bundle {
	route(path, (CallerScene, bundle))
}
/// Simply returns the caller as the scene to be rendered.
#[action(route)]
#[derive(Default, Component)]
#[require(Document)]
async fn CallerScene(cx: ActionContext<Request>) -> Result<SceneEntity> {
	SceneEntity::new_fixed(cx.id()).xok()
}


#[derive(Component, Reflect)]
#[require(Document, BlobSceneAction)]
pub struct BlobScene {
	path: RelPath,
}
impl BlobScene {
	pub fn new(path: impl Into<RelPath>) -> Self { Self { path: path.into() } }
}


#[action(route)]
#[derive(Default, Component)]
async fn BlobSceneAction(cx: ActionContext<Request>) -> Result<SceneEntity> {
	let bucket = cx
		.caller
		.with_state::<AncestorQuery<&Bucket>, Bucket>(|entity, query| {
			query
				.get(entity)
				.cloned()
				.unwrap_or_else(|_| Bucket::new(FsBucket::default()))
		})
		.await;

	let path = cx.caller.get::<BlobScene, _>(|fs| fs.path.clone()).await?;
	let bytes = bucket.get_media(&path).await?;

	cx.caller
		.with_then(move |mut entity_mut| {
			MediaParser::new().parse(ParseContext::new(&mut entity_mut, &bytes))
		})
		.await?;
	SceneEntity::new_fixed(cx.id()).xok()
}

/// Convenience function to create a simple route from a path and bundle.
pub fn route<B: Bundle>(path: &str, bundle: B) -> (PathPartial, B) {
	(PathPartial::new(path), bundle)
}
