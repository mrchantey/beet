use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Creates a router bundle with logging, help, and navigate middleware.
///
/// This is the standard way to set up routing. It includes:
/// - [`Router`] for route lookup and dispatch
/// - [`RequestLogger`] middleware for per-request logging
/// - [`HelpHandler`] middleware for `--help` support
/// - [`NavigateHandler`] middleware for `--navigate` support
///
/// All components are [`Reflect`] so the bundle round-trips through a scene.
pub fn router() -> impl Bundle {
	(
		Router,
		RequestLogger::default(),
		HelpHandler::default(),
		NavigateHandler::default(),
	)
}

/// Routes a request to the matching action in the [`RouteTree`],
/// applying ancestor [`MiddlewareList`] around the matched action.
///
/// When no route matches, renders contextual not-found help.
/// Middleware such as [`HelpHandler`] and [`NavigateHandler`] wrap
/// the inner action so they can intercept before dispatch.
#[action(handler_only)]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn Router(cx: ActionContext<Request>) -> Response {
	let caller = cx.caller.clone();
	let world = cx.world();
	let mut request = cx.input;
	let path = request.path().clone();

	// find the matching route in the tree
	let node = world
		.with_state::<AncestorQuery<&RouteTree>, _>(move |query| {
			query.get(caller.id()).map(|tree| tree.find(&path).cloned()).map_err(|_|{
				bevyhow!("Route tree not found. Was the `ActionMeta` added? was the `RouterPlugin` added?")
			})
		})
		.await;

	// resolve the inner action and dispatch entity from the matched route
	let (inner_action, dispatch_entity) = match &node {
		Ok(Some(node)) => {
			// surface matched dynamic segments (`:id`) to the handler
			node.merge_path_params(&mut request);
			let entity = world.entity(node.entity);
			match entity.clone().get_cloned::<ExchangeAction>().await {
				Ok(action) => (action.into_action(), entity),
				Err(err) => return err.into_response(),
			}
		}
		Ok(None) => {
			// no matching route — build a not-found response through
			// the contextual help system so middleware still applies
			(ContextualNotFound.into_action(), cx.caller.clone())
		}
		Err(err) => return bevyhow!("{err}").into_response(),
	};

	dispatch_entity
		.call_with_middleware(inner_action, request)
		.await
		.unwrap_or_else(|err| err.into_response())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	/// Test handler that echoes all request params as `key=v1/v2` pairs,
	/// sorted for deterministic output.
	#[action(handler_only)]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn EchoParams(cx: ActionContext<RequestParts>) -> MediaBytes {
		let mut pairs = cx
			.input
			.params()
			.iter_all()
			.map(|(key, values)| format!("{key}={}", values.join("/")))
			.collect::<Vec<_>>();
		pairs.sort();
		MediaBytes::new_text(pairs.join("&"))
	}

	#[beet_core::test]
	async fn dynamic_segment_reaches_handler() {
		router_world()
			.spawn((router(), children![exchange_route(
				"users/:id",
				EchoParams
			)]))
			.call::<Request, Response>(Request::get("users/42"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("id=42");
	}

	#[beet_core::test]
	async fn greedy_segment_reaches_handler() {
		router_world()
			.spawn((router(), children![exchange_route(
				"files/*path",
				EchoParams
			)]))
			.call::<Request, Response>(Request::get("files/a/b/c.txt"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("path=a/b/c.txt");
	}

	#[beet_core::test]
	async fn path_param_wins_over_query_param() {
		router_world()
			.spawn((router(), children![exchange_route(
				"users/:id",
				EchoParams
			)]))
			.call::<Request, Response>(Request::get("users/42?id=99"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("id=42")
			.xnot()
			.xpect_contains("99");
	}

	#[beet_core::test]
	async fn route_renders_scene() {
		router_world()
			.spawn((router(), children![render_action::fixed_route(
				"about",
				rsx_direct!{ <p>"About page"</p> }
			),]))
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
		router_world()
			.spawn((router(), children![render_action::fixed_route(
				"",
				rsx_direct!{ <p>"Root content"</p> }
			),]))
			.call::<Request, Response>(Request::get(""))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Root content");
	}

	#[beet_core::test]
	async fn route_renders_root_scene_child() {
		let body = router_world()
			.spawn((router(), children![
				render_action::fixed_route(
					"",
					rsx_direct!{ <h1>"My Server"</h1> <p>"welcome!"</p> }
				),
				render_action::fixed_route("about", rsx_direct!{ <p>"about"</p> }),
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
	async fn help_flag_returns_route_list() {
		router_world()
			.spawn((router(), children![
				increment(FieldRef::new("count")),
				render_action::fixed_route("about", rsx_direct!{ <p>"about"</p> }),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn dispatches_help_request() {
		router_world()
			.spawn((router(), children![
				increment(FieldRef::new("count")),
				render_action::fixed_route("about", rsx_direct!{ <p>"about"</p> }),
			]))
			.call::<Request, Response>(Request::from_cli_str("--help"))
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn not_found() {
		router_world()
			.spawn((router(), children![increment(FieldRef::new("count")),]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent"),
			)
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[beet_core::test]
	async fn renders_root_scene_on_empty_args() {
		router_world()
			.spawn((router(), children![
				render_action::fixed_route(
					"",
					rsx_direct!{ <h1>"My Server"</h1> <p>"welcome!"</p> }
				),
				render_action::fixed_route("about", rsx_direct!{ <p>"about"</p> }),
			]))
			.call::<Request, Response>(Request::from_cli_str(""))
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
			.spawn((router(), children![
				(
					render_action::fixed_route(
						"counter",
						Element::new("p").with_inner_text("counter")
					),
					children![increment(FieldRef::new("count")),],
				),
				render_action::fixed_route("about", rsx_direct!{ <p>"about"</p> }),
			]))
			.flush();

		let res = world
			.entity_mut(root)
			.call::<Request, Response>(
				Request::from_cli_str("counter --help"),
			)
			.await
			.unwrap();

		let body = res.unwrap_str().await;
		body.contains("increment").xpect_true();
		body.contains("about").xpect_false();
	}

	#[beet_core::test]
	async fn not_found_shows_ancestor_help() {
		router_world()
			.spawn((router(), children![increment(FieldRef::new("count")),]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent"),
			)
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("not found")
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn not_found_shows_scoped_ancestor_help() {
		router_world()
			.spawn((router(), children![
				(
					render_action::fixed_route(
						"counter",
						Element::new("p").with_inner_text("counter")
					),
					children![increment(FieldRef::new("count")),],
				),
				render_action::fixed_route("about", rsx_direct!{ <p>"about"</p> }),
			]))
			.call::<Request, Response>(
				Request::from_cli_str("counter nonsense"),
			)
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("not found")
			.xpect_contains("increment")
			.xnot()
			.xpect_contains("about");
	}

	/// A route can stream Server-Sent Events by returning a streaming
	/// [`Response`] via [`sse_response`] — no special router needed.
	#[cfg(feature = "json")]
	#[beet_core::test]
	async fn sse_route_streams_events() {
		#[derive(serde::Serialize)]
		struct Tick {
			index: u32,
		}

		#[action(handler_only)]
		#[derive(Default, Clone, Component, Reflect)]
		#[reflect(Component)]
		async fn Ticks(_cx: ActionContext<RequestParts>) -> Response {
			sse_response(bevy::tasks::futures_lite::stream::iter(
				(0..3).map(|index| Ok(SseBody::message(Tick { index }))),
			))
		}

		router_world()
			.spawn((router(), children![exchange_route("ticks", Ticks)]))
			.call::<Request, Response>(Request::get("ticks"))
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("data: {\"index\":0}")
			.xpect_contains("data: {\"index\":2}");
	}
}
