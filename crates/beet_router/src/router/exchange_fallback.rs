//! An interface routes requests to scenes and tools.
//!
//! The fallback chain skips children whose tool signature doesn't
//! match `Request → Response` (eg render tools that expect
//! different input types), so they can coexist on the same entity.
//!
//! This module provides [`default_router`], a request router that
//! handles routing, scene navigation, tool invocation, and help
//! rendering. It delegates to shared functions in [`help`] and
//! [`navigate`] rather than duplicating their logic.
//!
//! ## Routing Behavior
//!
//! - **Scenes**: tool-based routes created via [`scene_func`] that delegate
//!   rendering to the [`SceneToolRenderer`] on an ancestor.
//! - **`--help`**: scoped to the requested path prefix, ie
//!   `counter --help` only shows routes under `/counter`.
//! - **Not found**: shows help scoped to the nearest ancestor scene,
//!   ie `counter nonsense` shows help for `/counter`.
//!
//! ## Scene Rendering
//!
//! The `default_router` does **not** include a [`SceneToolRenderer`].
//! The `SceneToolRenderer` is the responsibility of the server, since
//! different servers need different rendering strategies.
//! Use `SceneToolRenderer::default()` for content-negotiated rendering.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_tool::prelude::*;

/// A Request/Response tool that will try each child until an
/// Outcome::Response is reached, or else returns a NotFound.
/// Errors are converted to a response.
///
/// Children whose tool signature doesn't match `Request →
/// Outcome<Response, Request>` are silently skipped so they can
/// coexist on the same entity without causing fallback errors.
/// This allows for the co-existence of the router children and the routes themselves.
pub fn exchange_fallback() -> impl Bundle {
	let fallback = Fallback::<Request, Response>::default()
		.with_exclude_errors(ChildError::NO_TOOL | ChildError::TOOL_MISMATCH);
	(
		RouteHidden,
		async_tool(async move |cx: ToolContext<Request>| -> Result<Response> {
			match fallback.run(cx).await? {
				Pass(res) => Ok(res),
				// no child matched — return a simple plaintext not-found
				Fail(req) => Ok(Response::from_status_body(
					StatusCode::NOT_FOUND,
					format!("Resource not found: {}", req.path_string()),
					MediaType::Text,
				)),
			}
		}),
	)
}

/// Creates a standard router with help, navigation, routing, and
/// fallback handlers as a child fallback chain.
///
/// Does **not** include a render tool — that belongs on the server.
///
/// The handler chain runs in order:
/// 1. **Help** — if `--help` is present, render help scoped to the
///    request path prefix.
/// 2. **Navigate** — if `--navigate` is present, resolve the
///    navigation direction relative to the current path.
/// 3. **Router** — look up the path in the [`RouteTree`]. All routes
///    are tools; scenes delegate to the render tool internally.
/// 4. **Contextual Not Found** — show help for the nearest ancestor
///    scene of the unmatched path.
pub fn default_router() -> impl Bundle {
	// use on_spawn to avoid clobbering children!
	(
		exchange_fallback(),
		OnSpawn::insert_child((
			Name::new("Help Tool"),
			RouteHidden,
			HelpHandler,
		)),
		OnSpawn::insert_child((
			Name::new("Navigate Tool"),
			RouteHidden,
			NavigateHandler,
		)),
		OnSpawn::insert_child(try_router()),
		OnSpawn::insert_child((
			Name::new("Contextual Not Found"),
			RouteHidden,
			ContextualNotFoundHandler,
		)),
	)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_node::prelude::*;
	use beet_tool::prelude::*;
	use bevy::ecs::entity::EntityHashMap;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	fn my_interface() -> impl Bundle {
		(
			system_tool(
				|In(req): In<ToolContext<Request>>,
				 trees: Query<&RouteTree>|
				 -> Result<RouteTree> {
					let tree = trees.get(req.id())?;
					Ok(tree.clone())
				},
			),
			children![(
				PathPartial::new("add"),
				func_tool(|input: ToolContext<(u32, u32)>| Ok(
					input.0 + input.1
				)),
			)],
		)
	}


	#[beet_core::test]
	async fn works() {
		router_world()
			.spawn(my_interface())
			.call::<_, RouteTree>(Request::get("foo"))
			.await
			.unwrap()
			.find(&["add"])
			.unwrap()
			.path
			.annotated_rel_path()
			.to_string()
			.xpect_eq("add");
	}

	#[beet_core::test]
	#[cfg(feature = "json")]
	async fn dispatches_tool_request() {
		router_world()
			.spawn((SceneToolRenderer::default(), default_router(), children![
				route_tool(
					"add",
					func_tool(|input: ToolContext<(i32, i32)>| Ok(
						input.0 + input.1
					))
				),
			]))
			.call::<Request, Response>(
				Request::with_json("add", &(1i32, 2i32)).unwrap(),
			)
			.await
			.unwrap()
			.json::<i32>()
			.await
			.unwrap()
			.xpect_eq(3);
	}

	#[beet_core::test]
	async fn help_flag_returns_route_list() {
		router_world()
			.spawn((SceneToolRenderer::default(), default_router(), children![
				increment(FieldRef::new("count")),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn dispatches_help_request() {
		router_world()
			.spawn((SceneToolRenderer::default(), default_router(), children![
				increment(FieldRef::new("count")),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help").unwrap())
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn not_found() {
		router_world()
			.spawn((SceneToolRenderer::default(), default_router(), children![
				increment(FieldRef::new("count")),
			]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent").unwrap(),
			)
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[beet_core::test]
	async fn renders_root_scene_on_empty_args() {
		router_world()
			.spawn((SceneToolRenderer::default(), default_router(), children![
				scene_func("", || {
					children![
						(Element::new("h1"), children![Value::Str(
							"My Server".into()
						)]),
						(Element::new("p"), children![Value::Str(
							"welcome!".into()
						)]),
					]
				}),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
			]))
			.call::<Request, Response>(Request::from_cli_str("").unwrap())
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("My Server")
			.xpect_contains("welcome!");
	}

	#[beet_core::test]
	async fn scoped_help_for_subcommand() {
		let mut world = router_world();

		let root = world
			.spawn((SceneToolRenderer::default(), default_router(), children![
				(
					scene_func("counter", || {
						(Element::new("p"), children![Value::Str(
							"counter".into()
						)])
					}),
					children![increment(FieldRef::new("count")),],
				),
				scene_func("about", || {
					(Element::new("p"), children![Value::Str("about".into())])
				}),
			]))
			.flush();

		let res = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("counter --help").unwrap(),
			)
			.await
			.unwrap();

		let body = res.unwrap_str().await;
		body.contains("increment").xpect_true();
		body.contains("about").xpect_false();
	}

	#[test]
	fn roundtrip_router_tools_scene() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init_plugin::<RouterAppPlugin>();
		app.init();
		app.update();

		let entity = app
			.world_mut()
			.spawn((Name::new("Helper"), HelpHandler))
			.id();

		// Serialize
		let scene = SceneSaver::new(app.world_mut())
			.with_entity_tree(entity)
			.save_ron()
			.unwrap();
		scene.xref().xpect_contains("HelpHandler");

		// Despawn original
		app.world_mut().entity_mut(entity).despawn();

		// Load
		let mut entity_map = EntityHashMap::default();
		SceneLoader::new(app.world_mut())
			.with_entity_map(&mut entity_map)
			.load_ron(&scene)
			.unwrap();
		app.update();

		// Verify the loaded entity has the component
		let loaded = *entity_map.values().next().unwrap();
		app.world().entity(loaded).get::<HelpHandler>().xpect_some();
	}
}
