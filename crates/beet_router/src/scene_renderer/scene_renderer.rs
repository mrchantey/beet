use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;

/// A single content container, similar to pages in a website or cards
/// in HyperCard. Each scene route is a route, with the exact rendering
/// behavior determined by the [`SceneToolRenderer`] on the server.
///
/// Use [`fixed_scene`] or [`scene_tool`] to create a routable scene.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[require(DocumentScope)]
pub struct SceneRoute;

/// Component on a server entity that controls how scenes are rendered.
/// When absent, the default content-negotiated renderer via
/// [`MediaRenderer`] is used as a fallback.
#[derive(Debug, Clone, Component)]
pub struct SceneToolRenderer {
	tool: Tool<RequestParts, Response>,
}

impl Default for SceneToolRenderer {
	fn default() -> Self {
		Self {
			tool: Tool::new_async(default_scene_renderer),
		}
	}
}

impl SceneToolRenderer {
	/// Creates a renderer with a custom tool.
	pub fn new(tool: Tool<RequestParts, Response>) -> Self { Self { tool } }

	/// Renders the given scene entity using the ancestor
	/// [`SceneToolRenderer`], falling back to the default renderer
	/// when none is found. Entities marked with [`DespawnOnRender`]
	/// are cleaned up after rendering.
	pub async fn render_entity(
		caller: &AsyncEntity,
		scene_entity: Entity,
		parts: RequestParts,
	) -> Result<Response> {
		let render_tool = caller
			.with_state::<AncestorQuery<&SceneToolRenderer>, _>(
				|entity, state| {
					state
						.get(entity)
						.cloned()
						.map(|renderer| renderer.into_tool())
				},
			)
			.await
			.unwrap_or_else(|_| Tool::new_async(default_scene_renderer));

		let scene_entity = caller.world().entity(scene_entity);
		let result = scene_entity.call_detached(render_tool, parts).await;

		if scene_entity.contains::<DespawnOnRender>().await {
			scene_entity.despawn().await;
		}

		result
	}
}

impl IntoTool<Self> for SceneToolRenderer {
	type In = RequestParts;
	type Out = Response;
	fn into_tool(self) -> Tool<RequestParts, Response> { self.tool }
}


/// Marker component for scene entities that should be despawned
/// after they render.
#[derive(Component)]
pub struct DespawnOnRender;


/// Creates a fixed routable scene from a path and content closure.
///
/// This approach is convenient
///
/// A scene func is a regular tool (`Tool<(), Entity>`) that calls the
/// closure, spawns an entity with the resulting bundle, and returns
/// the entity id. The [`ExchangeTool`] handles the
/// `Request` → `Response` conversion via [`SceneToolRenderer`].
///
/// # Example
///
/// ```no_run
/// use beet_router::prelude::*;
/// use beet_core::prelude::*;
/// use beet_node::prelude::*;
///
/// let bundle = fixed_scene("about", || {
///     Element::new("p").with_inner_text("About page")
/// });
/// ```
pub fn fixed_scene<F, B>(path: &str, func: F) -> impl Bundle
where
	F: 'static + Send + Sync + Clone + Fn() -> B,
	B: 'static + Send + Sync + Bundle,
{
	route(path, (CallerScene, func()))
}
/// Simply returns the caller as the scene to be rendered
#[tool(route)]
#[derive(Default, Component)]
#[require(SceneRoute)]
async fn CallerScene(cx: ToolContext<Request>) -> Result<SceneEntity> {
	SceneEntity::new_fixed(cx.id()).xok()
}


#[derive(Component, Reflect)]
#[require(SceneRoute, FileSceneTool)]
pub struct FileScene {
	path: WsPathBuf,
}
impl FileScene {
	pub fn new(path: impl Into<WsPathBuf>) -> Self {
		Self { path: path.into() }
	}
}


#[tool(route)]
#[derive(Default, Component)]
async fn FileSceneTool(cx: ToolContext<Request>) -> Result<SceneEntity> {
	let abs_path = cx
		.caller
		.get::<FileScene, _>(|fs| fs.path.into_abs())
		.await?;

	let media_type = MediaType::from_path(&abs_path);
	let bytes = fs_ext::read_async(&abs_path).await?;
	let bytes = MediaBytes::new(media_type, bytes);
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
