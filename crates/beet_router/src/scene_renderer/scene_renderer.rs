use crate::prelude::*;
use beet_net::prelude::*;
use beet_node::prelude::*;
use beet_tool::prelude::*;


#[derive(Debug, Clone, Component)]
pub struct SceneToolRenderer {
	tool: Tool<RequestParts, Response>,
}

impl Default for SceneToolRenderer {
	fn default() -> Self {
		Self {
			tool: async_tool(default_scene_renderer),
		}
	}
}


impl SceneToolRenderer {
	pub fn new(tool: Tool<RequestParts, Response>) -> Self { Self { tool } }
}


/// Call the provided tool on a spawned entity.
/// In the case of 'diffing' techniques its perfectly
/// acceptable for the tool to perform its diff then
/// return a unit bundle `()` which is a noop on insertion.
pub fn scene_tool<M, B>(
	path: &str,
	tool: impl IntoTool<M, In = Request, Out = B>,
) -> (PathPartial, Tool<Request, Response>)
where
	B: 'static + Send + Sync + Bundle,
{
	let tool = tool.into_tool();
	(
		PathPartial::new(path),
		async_tool(async move |cx: AsyncToolIn<Request>| -> Result<Response> {
			// 1. clone the parts for the renderer
			let parts = cx.input.parts().clone();

			// 2. spawn the entity to be renderer to.
			let entity = cx.caller.world().spawn_then(()).await;

			// 3. insert the bundle
			let bundle = cx.caller.call_detached(tool, cx.input).await?;
			entity.insert_then(bundle).await;

			let renderer = cx
				.caller
				.with_state::<AncestorQuery<&SceneToolRenderer>, _>(
					|entity, state| state.get(entity).cloned(),
				)
				.await?;
			let response = entity.call_detached(renderer.tool, parts).await;

			entity.despawn().await;

			response
		}),
	)
}


pub fn file_scene_tool(
	path: &str,
	file_path: impl Into<WsPathBuf>,
) -> (PathPartial, Tool<Request, Response>) {
	let ws_path: WsPathBuf = file_path.into();

	scene_tool(
		path,
		async_tool(async move |req| {
			let abs_path = ws_path.into_abs();
			let bytes = fs_ext::read_async(&abs_path).await?;

			let bytes = MediaBytes::new(MediaType::from_path(ws_path), bytes);

			req.caller
				.with_then(move |mut entity| {
					let mut parser = MediaParser::new();
					parser.parse(ParseContext::new(&mut entity, &bytes))
				})
				.await?;

			Ok(())
		}),
	)
}
