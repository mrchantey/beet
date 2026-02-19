use crate::prelude::*;
use beet_core::prelude::*;

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
		// the router itself shouldnt show up in the route tree
		RouteHidden,
		exchange_fallback(),
		OnSpawn::insert_child((RouteHidden, async_tool(router_tool))),
	)
}


/// Routes a request to the matching tool in the [`RouteTree`].
///
/// Looks up the request path in the [`RouteTree`], then forwards the
/// request to the matching tool via `entity.call`. Cards are now
/// regular tools that delegate to the render tool internally, so
/// no special handling is needed here.
async fn router_tool(
	cx: AsyncToolIn<Request>,
) -> Result<Outcome<Response, Request>> {
	let path = cx.input.path().clone();
	let tool_entity = cx.tool.id();
	let world = cx.tool.world();

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
	use super::*;



	#[beet_core::test]
	async fn route_renders_card() {
		StackPlugin::world()
			.spawn((router(), children![
				markdown_render_tool(),
				card("about", || Paragraph::with_text("About page")),
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
	async fn route_renders_root_card_on_empty_path() {
		StackPlugin::world()
			.spawn((router(), children![
				markdown_render_tool(),
				card("", || Paragraph::with_text("Root content")),
			]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Root content");
	}

	#[beet_core::test]
	async fn route_renders_root_card_child() {
		let body = StackPlugin::world()
			.spawn((router(), children![
				markdown_render_tool(),
				card("", || {
					children![
						Heading1::with_text("My Server"),
						Paragraph::with_text("welcome!"),
					]
				}),
				card("about", || Paragraph::with_text("about")),
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
	async fn route_calls_route_tool() {
		StackPlugin::world()
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
