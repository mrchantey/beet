use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;

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
	/// when none is found. Entities in `despawn` are cleaned up
	/// after rendering.
	pub async fn render_entity(
		caller: &AsyncEntity,
		scene: SceneEntity,
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

		let scene_entity = caller.world().entity(scene.entity);
		let result = scene_entity.call_detached(render_tool, parts).await;

		// despawn all ephemeral entities
		let world = caller.world();
		for entity in scene.despawn {
			world.entity(entity).despawn().await;
		}

		result
	}
}

impl IntoTool<Self> for SceneToolRenderer {
	type In = RequestParts;
	type Out = Response;
	fn into_tool(self) -> Tool<RequestParts, Response> { self.tool }
}


/// Creates a fixed routable scene from a path and content bundle.
///
/// The entity itself becomes both the route and the scene content.
/// The [`ExchangeTool`] handles the `Request` → `Response`
/// conversion via [`SceneToolRenderer`].
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
#[tool(route)]
#[derive(Default, Component)]
#[require(DocumentScope)]
async fn CallerScene(cx: ToolContext<Request>) -> Result<SceneEntity> {
	SceneEntity::new_fixed(cx.id()).xok()
}


#[derive(Component, Reflect)]
#[require(DocumentScope, FileSceneTool)]
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
