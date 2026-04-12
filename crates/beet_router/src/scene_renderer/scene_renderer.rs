use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;

/// A single content container, similar to pages in a website or cards
/// in HyperCard. Each scene route is a route, with the exact rendering
/// behavior determined by the [`SceneToolRenderer`] on the server.
///
/// Use [`scene_func`] or [`scene_tool`] to create a routable scene.
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


/// Creates a routable scene tool from a path and a tool that returns
/// a [`Bundle`].
///
/// A scene tool is a regular tool (`Tool<(), Entity>`) that spawns an
/// entity with the provided bundle and returns its id. The
/// [`ExchangeTool`] handles request extraction and renders the
/// entity via [`SceneToolRenderer`].
///
/// # Example
///
/// ```no_run
/// use beet_router::prelude::*;
/// use beet_core::prelude::*;
/// use beet_net::prelude::*;
/// use beet_node::prelude::*;
/// use beet_tool::prelude::*;
///
/// let bundle = scene_tool("about", Tool::<Request, (Element,)>::new_pure(
///     |_cx| Ok((Element::new("p"),))
/// ));
/// ```
pub fn scene_tool<M, B>(
	path: &str,
	tool: impl IntoTool<M, In = Request, Out = B>,
) -> impl Bundle
where
	B: 'static + Send + Sync + Bundle,
{
	let inner = tool.into_tool();

	// The entity's own tool: spawns a scene entity, inserts the bundle,
	// and returns the entity id.
	let scene_spawner = Tool::<(), Entity>::new_async(
		async move |cx: ToolContext<()>| -> Result<Entity> {
			let bundle =
				cx.caller.call_detached(inner, Request::get("")).await?;
			let entity = cx.world().spawn_then((DespawnOnRender, bundle)).await;
			entity.id().xok()
		},
	);

	// Capture the spawner so the exchange tool can call it via call_detached.
	// cx.caller is the root entity (from Router2 dispatch), not the route
	// entity, so we must use call_detached with the captured tool.
	let spawner_for_exchange = scene_spawner.clone();
	let exchange = ExchangeTool::from_tool(Tool::new_async(
		async move |cx: ToolContext<Request>| -> Result<Response> {
			let parts = cx.input.parts().clone();
			let entity: Entity = cx
				.caller
				.call_detached(spawner_for_exchange.clone(), ())
				.await?;
			SceneToolRenderer::render_entity(&cx.caller, entity, parts).await
		},
	));

	(PathPartial::new(path), SceneRoute, scene_spawner, exchange)
}

/// Creates a routable scene from a path and content closure.
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
/// let bundle = scene_func("about", || {
///     Element::new("p").with_inner_text("About page")
/// });
/// ```
pub fn scene_func<F, B>(path: &str, func: F) -> impl Bundle
where
	F: 'static + Send + Sync + Clone + Fn() -> B,
	B: 'static + Send + Sync + Bundle,
{
	// The entity's own tool: calls the closure and spawns the result
	let scene_spawner = Tool::<(), Entity>::new_async(
		async move |cx: ToolContext<()>| -> Result<Entity> {
			let entity = cx.world().spawn_then((DespawnOnRender, func())).await;
			entity.id().xok()
		},
	);

	// Capture the spawner so the exchange tool can call it via call_detached.
	// cx.caller is the root entity (from Router2 dispatch), not the route
	// entity, so we must use call_detached with the captured tool.
	let spawner_for_exchange = scene_spawner.clone();
	let exchange = ExchangeTool::from_tool(Tool::new_async(
		async move |cx: ToolContext<Request>| -> Result<Response> {
			let parts = cx.input.parts().clone();
			let entity: Entity = cx
				.caller
				.call_detached(spawner_for_exchange.clone(), ())
				.await?;
			SceneToolRenderer::render_entity(&cx.caller, entity, parts).await
		},
	));

	(PathPartial::new(path), SceneRoute, scene_spawner, exchange)
}

#[derive(Component, Reflect)]
#[require(FileSceneTool)]
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
	SceneEntity(cx.id()).xok()
}



/// Creates a routable scene that loads and parses a file.
///
/// A file scene tool is a regular tool (`Tool<(), Entity>`) that reads
/// the file, parses its content via [`MediaParser`] onto the caller
/// entity, and returns the caller's id. The [`ExchangeTool`] handles
/// `Request` → `Response` conversion via [`SceneToolRenderer`].
///
/// # Example
///
/// ```no_run
/// use beet_router::prelude::*;
/// use beet_core::prelude::*;
///
/// let bundle = file_scene_tool("readme", "docs/readme.md");
/// ```
pub fn file_scene_tool(
	path: &str,
	file_path: impl Into<WsPathBuf>,
) -> impl Bundle {
	let ws_path: WsPathBuf = file_path.into();

	// The entity's own tool: reads file, parses onto a fresh entity,
	// returns its id
	let ws_path_inner = ws_path.clone();
	let scene_spawner = Tool::<(), Entity>::new_async(
		async move |cx: ToolContext<()>| -> Result<Entity> {
			// read the file
			let abs_path = ws_path_inner.clone().into_abs();
			let media_type = MediaType::from_path(&ws_path_inner);
			let bytes = fs_ext::read_async(&abs_path).await?;
			let bytes = MediaBytes::new(media_type, bytes);

			// spawn entity and parse content onto it
			let entity = cx.world().spawn_then(DespawnOnRender).await;
			entity
				.with_then(move |mut entity_mut| {
					MediaParser::new()
						.parse(ParseContext::new(&mut entity_mut, &bytes))
				})
				.await?;
			entity.id().xok()
		},
	);

	// Capture the spawner so the exchange tool can call it via call_detached.
	// cx.caller is the root entity (from Router2 dispatch), not the route
	// entity, so we must use call_detached with the captured tool.
	let spawner_for_exchange = scene_spawner.clone();
	let exchange = ExchangeTool::from_tool(Tool::new_async(
		async move |cx: ToolContext<Request>| -> Result<Response> {
			let parts = cx.input.parts().clone();
			let entity: Entity = cx
				.caller
				.call_detached(spawner_for_exchange.clone(), ())
				.await?;
			SceneToolRenderer::render_entity(&cx.caller, entity, parts).await
		},
	));

	(PathPartial::new(path), SceneRoute, scene_spawner, exchange)
}

/// Convenience function to create a simple route from a path and bundle.
pub fn route<B: Bundle>(path: &str, bundle: B) -> (PathPartial, B) {
	(PathPartial::new(path), bundle)
}
