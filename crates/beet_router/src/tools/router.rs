use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_tool::prelude::*;

/// Fallback tool for mapping a request path to a corresponding tool in this
/// tree's hierarchy, using the [`RouteTree`] in its ancestors.
pub fn try_router() -> impl Bundle {
	(Name::new("Router"), RouteHidden, async_tool(router_tool))
}

/// Fallback tool for mapping a request path to a corresponding tool in this
/// tree's hierarchy, using the [`RouteTree`] in its ancestors.
pub fn router() -> impl Bundle {
	(
		Name::new("Router"),
		exchange_fallback(),
		OnSpawn::insert_child((RouteHidden, async_tool(router_tool))),
	)
}

/// Routes a request to the matching tool in the [`RouteTree`].
///
/// Looks up the request path in the [`RouteTree`], then forwards the
/// request to the matching tool via `entity.call`. Scene routes are
/// regular tools that delegate to the render tool internally, so
/// no special handling is needed here.
async fn router_tool(
	cx: AsyncToolIn<Request>,
) -> Result<Outcome<Response, Request>> {
	let path = cx.input.path().clone();
	let tool_entity = cx.caller.id();
	let world = cx.caller.world();

	let node = world
		.with_then(move |world: &mut World| -> Result<Option<ToolNode>> {
			// no tree is a real error
			let tree = root_route_tree(world, tool_entity)?;
			tree.find(&path).cloned().xok()
		})
		.await?;

	match node {
		Some(tool_node) => Pass(
			world
				.entity(tool_node.entity)
				.call::<Request, Response>(cx.input)
				.await?,
		),
		None => Fail(cx.input),
	}
	.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_node::prelude::*;
	use beet_tool::prelude::*;

	/// A simple render tool for tests that spawns the scene content,
	/// collects [`Value`] text from children, and returns it as a
	/// plain-text response.
	fn test_render_tool() -> impl Bundle {
		(
			Name::new("Test Render Tool"),
			RenderToolMarker,
			RouteHidden,
			async_tool(
				async |cx: AsyncToolIn<RenderRequest>| -> Result<Response> {
					let spawn_tool = cx.input.spawn_tool.clone();
					let world = cx.caller.world();

					let scene_entity =
						cx.caller.call_detached(spawn_tool, ()).await?;

					let text = world
						.with_then(move |world: &mut World| -> String {
							collect_scene_text(world, scene_entity)
						})
						.await;

					Response::ok_body(text, MediaType::Text).xok()
				},
			),
		)
	}

	/// Recursively collect text from [`Value::Str`] components in an
	/// entity tree, then despawn the root.
	fn collect_scene_text(world: &mut World, entity: Entity) -> String {
		let mut parts = Vec::new();
		collect_text_recursive(world, entity, &mut parts);
		world.entity_mut(entity).despawn();
		parts.join("")
	}

	fn collect_text_recursive(
		world: &World,
		entity: Entity,
		parts: &mut Vec<String>,
	) {
		if let Some(value) = world.entity(entity).get::<Value>() {
			if let Value::Str(text) = value {
				parts.push(text.clone());
			}
		}
		if let Some(children) = world.entity(entity).get::<Children>() {
			for child in children.iter() {
				collect_text_recursive(world, child, parts);
			}
		}
	}

	#[beet_core::test]
	async fn route_renders_scene() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), children![
				test_render_tool(),
				scene_route("about", || {
					(Element::new("p"), children![Value::Str(
						"About page".into()
					)])
				}),
			]))
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.contains("About page")
			.xpect_true();
	}

	#[beet_core::test]
	async fn route_renders_root_scene_on_empty_path() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), children![
				test_render_tool(),
				scene_route("", || {
					(Element::new("p"), children![Value::Str(
						"Root content".into()
					)])
				}),
			]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Root content");
	}

	#[beet_core::test]
	async fn route_renders_root_scene_child() {
		let body = (AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), children![
				test_render_tool(),
				scene_route("", || {
					children![
						(Element::new("h1"), children![Value::Str(
							"My Server".into()
						)]),
						(Element::new("p"), children![Value::Str(
							"welcome!".into()
						)]),
					]
				}),
				scene_route("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
			]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		body.contains("My Server").xpect_true();
		body.contains("welcome!").xpect_true();
	}

	#[beet_core::test]
	#[cfg(feature = "json")]
	async fn route_calls_route_tool() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), children![route_tool(
				"add",
				func_tool(
					|input: FuncToolIn<(i32, i32)>| Ok(input.0 + input.1)
				),
			),]))
			.call::<Request, Response>(
				Request::with_json("/add", &(10i32, 20i32)).unwrap(),
			)
			.await
			.unwrap()
			.body
			.into_json::<i32>()
			.await
			.unwrap()
			.xpect_eq(30);
	}
}
