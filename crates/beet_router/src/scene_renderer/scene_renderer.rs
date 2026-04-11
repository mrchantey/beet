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
/// Each server must have a [`SceneToolRenderer`] at its root or on an
/// ancestor of scene tool entities. Use [`SceneToolRenderer::default`]
/// for standard content-negotiated rendering via [`MediaRenderer`].
#[derive(Debug, Clone, Component)]
pub struct SceneToolRenderer {
	tool: Tool<RequestParts, Response>,
}

impl Default for SceneToolRenderer {
	fn default() -> Self {
		Self {
			tool: Tool::<RequestParts, Response>::new_async(
				default_scene_renderer,
			),
		}
	}
}

impl SceneToolRenderer {
	/// Creates a renderer with a custom tool.
	pub fn new(tool: Tool<RequestParts, Response>) -> Self { Self { tool } }
}

impl IntoTool<Self> for SceneToolRenderer {
	type In = RequestParts;
	type Out = Response;
	fn into_tool(self) -> Tool<RequestParts, Response> { self.tool }
}


/// Creates a routable scene tool from a path and a tool that returns
/// a [`Bundle`].
///
/// The tool receives the full [`Request`] and returns a bundle to insert
/// on the spawned scene entity. For tools that modify the entity
/// in-place (like parsers), return `()` as a noop bundle.
///
/// Rendering is handled by the [`SceneToolRenderer`] found on an
/// ancestor entity.
pub fn scene_tool<M, B>(
	path: &str,
	tool: impl IntoTool<M, In = Request, Out = B>,
) -> (PathPartial, SceneRoute, Tool<Request, Response>)
where
	B: 'static + Send + Sync + Bundle,
{
	let tool = tool.into_tool();
	(
		PathPartial::new(path),
		SceneRoute,
		Tool::<Request, Response>::new_async(
			async move |cx: ToolContext<Request>| -> Result<Response> {
				// 1. clone the parts for the renderer
				let parts = cx.input.parts().clone();

				// 2. spawn the entity to be rendered to
				let entity = cx.world().spawn_then(()).await;

				// 3. call the inner tool and insert its bundle
				let bundle = cx.caller.call_detached(tool, cx.input).await?;
				entity.insert_then(bundle).await;

				// 4. find the renderer on an ancestor
				let renderer = cx
					.caller
					.with_state::<AncestorQuery<&SceneToolRenderer>, _>(
						|entity, state| state.get(entity).cloned(),
					)
					.await?;

				// 5. render and clean up
				let response = entity.call_detached(renderer.tool, parts).await;
				entity.despawn().await;
				response
			},
		),
	)
}

/// Creates a routable scene from a path and content closure.
///
/// Convenience wrapper around [`scene_tool`] for simple scenes that
/// don't need request data.
///
/// # Example
///
/// ```no_run
/// use beet_router::prelude::*;
/// use beet_core::prelude::*;
/// use beet_node::prelude::*;
///
/// let bundle = scene_func("about", || {
///     (Element::new("p"), children![Value::Str("About page".into())])
/// });
/// ```
pub fn scene_func<F, B>(path: &str, func: F) -> impl Bundle
where
	F: 'static + Send + Sync + Clone + Fn() -> B,
	B: 'static + Send + Sync + Bundle,
{
	scene_tool(
		path,
		Tool::<Request, B>::new_pure(move |_: ToolContext<Request>| Ok(func())),
	)
}

/// Creates a routable scene that loads and parses a file.
///
/// On each request the file is read from disk and parsed via
/// [`MediaParser`] into a semantic entity tree. The tree is then
/// rendered by the [`SceneToolRenderer`] found on an ancestor.
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
	(
		PathPartial::new(path),
		SceneRoute,
		Tool::<Request, Response>::new_async(
			async move |cx: ToolContext<Request>| -> Result<Response> {
				let parts = cx.input.parts().clone();

				// read file
				let abs_path = ws_path.clone().into_abs();
				let media_type = MediaType::from_path(&ws_path);
				let bytes = fs_ext::read_async(&abs_path).await?;
				let bytes = MediaBytes::new(media_type, bytes);

				// spawn entity and parse content onto it
				let entity = cx.world().spawn_then(()).await;
				entity
					.with_then(move |mut entity_mut| {
						let mut parser = MediaParser::new();
						parser.parse(ParseContext::new(&mut entity_mut, &bytes))
					})
					.await?;

				// find renderer on ancestor and render
				let renderer = cx
					.caller
					.with_state::<AncestorQuery<&SceneToolRenderer>, _>(
						|entity, state| state.get(entity).cloned(),
					)
					.await?;
				let response = entity.call_detached(renderer.tool, parts).await;
				entity.despawn().await;
				response
			},
		),
	)
}
