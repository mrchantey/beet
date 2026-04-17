use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;

/// Component on a server entity that controls how scenes are rendered.
/// When absent, the default content-negotiated renderer via
/// [`MediaRenderer`] is used as a fallback.
#[derive(Debug, Clone, Component)]
pub struct SceneActionRenderer {
	action: Action<RequestParts, Response>,
}

impl Default for SceneActionRenderer {
	fn default() -> Self {
		Self {
			action: Action::new_async(default_scene_renderer),
		}
	}
}

impl SceneActionRenderer {
	/// Creates a renderer with a custom action.
	pub fn new(action: Action<RequestParts, Response>) -> Self {
		Self { action }
	}

	async fn resolve(entity: &AsyncEntity) -> Action<RequestParts, Response> {
		entity
			.with_state::<AncestorQuery<&SceneActionRenderer>, _>(
				|entity, state| {
					state
						.get(entity)
						.cloned()
						.map(|renderer| renderer.into_action())
				},
			)
			.await
			.unwrap_or_else(|_| Action::new_async(default_scene_renderer))
	}

	/// Renders the given scene entity using the ancestor
	/// [`SceneActionRenderer`], falling back to the default renderer
	/// when none is found.
	///
	/// Before rendering, applies any ancestor scene middleware
	/// ([`MiddlewareList<RequestParts, SceneEntity>`]) to allow
	/// layout wrapping via slots. Entities in `despawn` are cleaned
	/// up after rendering.
	pub async fn render_entity(
		caller: &AsyncEntity,
		scene: SceneEntity,
		parts: RequestParts,
	) -> Result<Response> {
		// apply ancestor scene middleware (layout wrapping, etc.)
		let scene = Self::apply_scene_middleware(caller, scene, &parts).await?;

		// find the nearest ancestor SceneActionRenderer or fall back
		let render_action = Self::resolve(caller).await;

		let scene_entity = caller.world().entity(scene.entity);
		let result = scene_entity.call_detached(render_action, parts).await;

		// despawn all ephemeral entities
		let world = caller.world();
		for entity in scene.despawn {
			world.entity(entity).despawn().await;
		}

		result
	}

	/// Applies ancestor [`MiddlewareQuery`] middleware to the scene.
	///
	/// Creates an inner action that returns the original scene, wraps
	/// it with any ancestor `RequestParts/SceneEntity` middleware,
	/// and executes the chain. When no middleware exists the original
	/// scene is returned unchanged.
	async fn apply_scene_middleware(
		caller: &AsyncEntity,
		scene: SceneEntity,
		parts: &RequestParts,
	) -> Result<SceneEntity> {
		// check for ancestor scene middleware first
		let has_middleware = caller
			.with_state::<MiddlewareQuery<RequestParts, SceneEntity>, _>(
				|entity, query| query.has_middleware(entity),
			)
			.await;
		if !has_middleware {
			return Ok(scene);
		}

		// build inner action that returns the original scene
		let inner: Action<RequestParts, SceneEntity> = Action::new_async({
			let scene = scene.clone();
			async move |_cx: ActionContext<RequestParts>| {
				scene.xok::<BevyError>()
			}
		});

		// wrap with ancestor scene middleware and execute
		let wrapped = caller
			.with_state::<MiddlewareQuery<RequestParts, SceneEntity>, _>({
				move |entity, query| query.resolve_action(entity, inner)
			})
			.await;

		caller.call_detached(wrapped, parts.clone()).await
	}
}

impl IntoAction<Self> for SceneActionRenderer {
	type In = RequestParts;
	type Out = Response;
	fn into_action(self) -> Action<RequestParts, Response> { self.action }
}


/// Creates a fixed routable scene from a path and content bundle.
///
/// The entity itself becomes both the route and the scene content.
/// The [`ExchangeAction`] handles the `Request` → `Response`
/// conversion via [`SceneActionRenderer`].
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
#[require(DocumentScope)]
async fn CallerScene(cx: ActionContext<Request>) -> Result<SceneEntity> {
	SceneEntity::new_fixed(cx.id()).xok()
}


#[derive(Component, Reflect)]
#[require(DocumentScope, BlobSceneAction)]
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
