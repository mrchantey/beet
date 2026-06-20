use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Markup component for an entry that routes: it occupies the entity's action
/// slot with an [`ActionTrigger`] (so the boot exchange fans out to any servers
/// present) and installs an [`ExchangeAction`] (the route-tree dispatch, reached
/// via [`exchange`](beet_net::prelude::AsyncExchangeExt::exchange)).
///
/// Splitting the two lets one host both fan a boot out (its slot) and dispatch
/// per-request (its [`ExchangeAction`]): a [`CliServer`] resolves the boot by
/// routing, an [`HttpServer`] parks and routes each socket request.
///
/// `Reflect` is derived unconditionally: reflection works on no_std and is wanted
/// there for scene loading.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(
	ActionTrigger<Request, Response>,
	ExchangeAction = ExchangeAction(route_action()),
)]
pub struct Router;

/// The route-tree dispatch behind a router's [`ExchangeAction`]: matches the request
/// against the ancestor [`RouteTree`] and applies ancestor [`MiddlewareList`]
/// around the matched action.
///
/// When no route matches, the std build renders contextual not-found help through
/// the beet_ui scene pipeline; the no_std build falls back to a plain-text `404`
/// listing the available routes. Middleware such as [`HelpHandler`] and
/// [`NavigateHandler`] wrap the inner action so they can intercept before dispatch.
pub fn route_action() -> Action<Request, Response> {
	Action::new_async(
		async move |cx: ActionContext<Request>| -> Result<Response> {
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
						Err(err) => return Ok(err.into_response()),
					}
				}
				Ok(None) => {
					// no matching route — std builds a not-found response through the
					// contextual help system so middleware still applies; no_std falls
					// back to a plain-text route listing (no scene pipeline).
					#[cfg(feature = "std")]
					let action = ContextualNotFound.into_action();
					#[cfg(not(feature = "std"))]
					let action = not_found_action();
					(action, cx.caller.clone())
				}
				Err(err) => return Ok(bevyhow!("{err}").into_response()),
			};

			dispatch_entity
				.call_with_middleware(inner_action, request)
				.await
				.unwrap_or_else(|err| err.into_response())
				.xok()
		},
	)
}

/// Builds the no_std not-found fallback: a plain-text `404` listing the
/// available routes, queried from the ancestor [`RouteTree`].
///
/// The std build instead uses `ContextualNotFound`, which renders the help
/// scene through the beet_ui pipeline.
#[cfg(not(feature = "std"))]
fn not_found_action() -> Action<Request, Response> {
	Action::new_async(
		async move |cx: ActionContext<Request>| -> Result<Response> {
			let path = cx.input.path_string();
			let fallback = format!("Route {path} not found.");
			let body = cx
				.caller
				.with_state::<AncestorQuery<&RouteTree>, String>(
					move |entity, query| match query.get(entity) {
						Ok(tree) => {
							format!(
								"Route {path} not found.\n\n{}",
								format_route_help(tree)
							)
						}
						Err(_) => format!("Route {path} not found."),
					},
				)
				.await
				.unwrap_or(fallback);
			let mut response = Response::ok().with_body(body);
			response.parts.status = StatusCode::NOT_FOUND;
			Ok(response)
		},
	)
}

/// Format a [`RouteTree`] as a plain-text route listing (no_std help fallback).
/// The `help` route itself is excluded from the listing.
#[cfg(not(feature = "std"))]
fn format_route_help(tree: &RouteTree) -> String {
	let mut output = String::from("Available routes:\n");
	let nodes: Vec<&ActionNode> = tree
		.flatten_nodes()
		.into_iter()
		.filter(|node| {
			node.path.annotated_path().last_segment() != Some("help")
		})
		.collect();
	if nodes.is_empty() {
		output.push_str("  (none)\n");
		return output;
	}
	for node in nodes {
		let path = node.path.annotated_path();
		match &node.method {
			Some(method) => output.push_str(&format!("  /{path} [{method}]\n")),
			None => output.push_str(&format!("  /{path}\n")),
		}
		if let Some(description) = node.description() {
			output.push_str(&format!("    {description}\n"));
		}
	}
	output
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
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
			.spawn((default_router(), children![exchange_route(
				"users/:id",
				EchoParams
			)]))
			.exchange(Request::get("users/42"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("id=42");
	}

	#[beet_core::test]
	async fn greedy_segment_reaches_handler() {
		router_world()
			.spawn((default_router(), children![exchange_route(
				"files/*path",
				EchoParams
			)]))
			.exchange(Request::get("files/a/b/c.txt"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("path=a/b/c.txt");
	}

	#[beet_core::test]
	async fn path_param_wins_over_query_param() {
		router_world()
			.spawn((default_router(), children![exchange_route(
				"users/:id",
				EchoParams
			)]))
			.exchange(Request::get("users/42?id=99"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("id=42")
			.xnot()
			.xpect_contains("99");
	}

	#[beet_core::test]
	async fn route_renders_scene() {
		router_world()
			.spawn((default_router(), children![
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"About page"</p> }
				),
			]))
			.exchange(Request::get("about"))
			.await
			.unwrap_str()
			.await
			.contains("About page")
			.xpect_true();
	}

	#[beet_core::test]
	async fn route_renders_root_scene_on_empty_path() {
		router_world()
			.spawn((default_router(), children![
				render_action::fixed_func_route(
					"",
					|| rsx! { <p>"Root content"</p> }
				),
			]))
			.exchange(Request::get(""))
			.await
			.unwrap_str()
			.await
			.xpect_contains("Root content");
	}

	#[beet_core::test]
	async fn route_renders_root_scene_child() {
		let body = router_world()
			.spawn((default_router(), children![
				render_action::fixed_func_route(
					"",
					|| rsx! { <h1>"My Server"</h1> <p>"welcome!"</p> }
				),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.exchange(Request::get(""))
			.await
			.unwrap_str()
			.await;
		body.contains("My Server").xpect_true();
		body.contains("welcome!").xpect_true();
	}

	#[beet_core::test]
	async fn help_flag_returns_route_list() {
		router_world()
			.spawn((default_router(), children![
				increment(FieldRef::new("count")),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.exchange(Request::from_cli_str("--help"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn dispatches_help_request() {
		router_world()
			.spawn((default_router(), children![
				increment(FieldRef::new("count")),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.exchange(Request::from_cli_str("--help"))
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn not_found() {
		router_world()
			.spawn((default_router(), children![increment(FieldRef::new(
				"count"
			)),]))
			.exchange(Request::from_cli_str("nonexistent"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}

	#[beet_core::test]
	async fn renders_root_scene_on_empty_args() {
		router_world()
			.spawn((default_router(), children![
				render_action::fixed_func_route(
					"",
					|| rsx! { <h1>"My Server"</h1> <p>"welcome!"</p> }
				),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.exchange(Request::from_cli_str(""))
			.await
			.unwrap_str()
			.await
			.xpect_contains("My Server")
			.xpect_contains("welcome!");
	}

	#[beet_core::test]
	async fn scoped_help_for_subcommand() {
		let mut world = router_world();

		let root = world
			.spawn((default_router(), children![
				(
					render_action::fixed_func_route("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![increment(FieldRef::new("count")),],
				),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.flush();

		let res = world
			.entity_mut(root)
			.exchange(Request::from_cli_str("counter --help"))
			.await;

		let body = res.unwrap_str().await;
		body.contains("increment").xpect_true();
		body.contains("about").xpect_false();
	}

	#[beet_core::test]
	async fn not_found_shows_ancestor_help() {
		router_world()
			.spawn((default_router(), children![increment(FieldRef::new(
				"count"
			)),]))
			.exchange(Request::from_cli_str("nonexistent"))
			.await
			.text()
			.await
			.unwrap()
			.xpect_contains("not found")
			.xpect_contains("Available routes");
	}

	#[beet_core::test]
	async fn not_found_shows_scoped_ancestor_help() {
		router_world()
			.spawn((default_router(), children![
				(
					render_action::fixed_func_route("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![increment(FieldRef::new("count")),],
				),
				render_action::fixed_func_route(
					"about",
					|| rsx! { <p>"about"</p> }
				),
			]))
			.exchange(Request::from_cli_str("counter nonsense"))
			.await
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
			.spawn((default_router(), children![exchange_route(
				"ticks", Ticks
			)]))
			.exchange(Request::get("ticks"))
			.await
			.text()
			.await
			.unwrap()
			.xpect_contains("data: {\"index\":0}")
			.xpect_contains("data: {\"index\":2}");
	}
}
