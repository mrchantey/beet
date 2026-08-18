use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Markup component for an entry that routes: it fills the entity's
/// `Action<Request, Response>` slot with [`Router::action`], the route-tree dispatch
/// reached via [`exchange`](beet_net::prelude::AsyncExchangeExt::exchange).
///
/// `Router` is pure dispatch and observes nothing; booting belongs to the server
/// it is a child of (a [`CliServer`] resolves its boot by routing down into this
/// dispatch, an [`HttpServer`] parks and routes each socket request into it).
///
/// Middleware is opt-in, [`HelpHandler`] included: a router that should answer
/// `--help` / `?help` declares it, as a spread in markup (`<Router {HelpHandler}>`)
/// or through [`Router::with_defaults`] in Rust. An api-only router is entitled to
/// serve no help at all.
///
/// `Reflect` is derived unconditionally: reflection works on no_std and is wanted
/// there for scene loading.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Action<Request, Response> = Router::action().with_meta(ActionMeta::of::<Self, Request, Response>()))]
#[component(on_add = Action::<Request, Response>::assert_provider::<Self>)]
pub struct Router;

/// The [`Router`] at or under `root`, the entity a request is dispatched to.
///
/// A built entry root is its *server*, which parks on its boot action rather than
/// dispatching, so a caller holding that root (static export, pdf export) addresses
/// the router beneath it. A bare router root resolves to itself.
///
/// # Errors
/// Errors when no [`Router`] is at or under `root`.
pub fn find_router(
	In(root): In<Entity>,
	children: Query<&Children>,
	routers: Query<(), With<Router>>,
) -> Result<Entity> {
	children
		.iter_descendants_inclusive(root)
		.find(|entity| routers.contains(*entity))
		.ok_or_else(|| bevyhow!("no Router at or under {root}"))
}

/// A markup-spawnable route: its `path` prop becomes a [`PathPartial`], and its
/// declared children slot in, so a handler and any sub-content can be nested inside.
///
/// The url and the behavior are separate concerns, both declared at the call site:
/// the `path` prop is the route pattern, and the handler rides a component spread on
/// the same entity. A greedy `*name?` segment captures every trailing path part, eg
/// `<Route path="docs/*rest?" {handler}/>` matches any path beneath `docs/`. (A
/// self-mounting handler that owns its prefix, like [`ServeBlobs`], needs no `<Route>`.)
///
/// The Rust equivalent is the [`route::new`](crate::prelude::route) helper. It is a
/// [`template`](macro@template) rather than a marker component, so it expands to a
/// [`PathPartial`] (carrying a default [`Slot`](beet_core::prelude::SlotTarget) for
/// the declared children) at build time, with no component left to re-fire on reload.
#[template]
pub fn Route(
	/// The route path pattern, eg `docs/*rest?`; defaults to the root.
	#[prop(into)]
	path: String,
) -> impl Bundle {
	(PathPartial::new(path), children![SlotTarget::new()])
}

impl Router {
	/// The route-tree dispatch behind a router's `Action<Request, Response>` slot:
	/// matches the request against the ancestor [`RouteTree`] and applies ancestor
	/// [`MiddlewareList`] around the matched action.
	///
	/// When no route matches, the std build renders contextual not-found help through
	/// the beet_ui scene pipeline; the no_std build falls back to a plain-text `404`
	/// listing the available routes. Middleware such as [`HelpHandler`] and
	/// [`NavigateHandler`] wrap the inner action so they can intercept before dispatch.
	pub fn action() -> Action<Request, Response> {
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
						// a route declaring a method answers 405 to any other,
						// rather than running the handler against a request shape
						// it never expected: a `GET` on the POST-only analytics
						// beacon reached the body parser and logged an internal
						// error for every bot probe.
						match node.method {
							Some(allowed)
								if !allowed.allows(request.method()) =>
							{
								return HttpError::from_status(
									StatusCode::METHOD_NOT_ALLOWED,
								)
								.xmap(Response::from)
								.xok();
							}
							_ => {}
						}
						// surface matched dynamic segments (`:id`) to the handler
						node.merge_path_params(&mut request);
						let entity = world.entity(node.entity);
						// dispatch through call resolution: the route's canonical
						// `Action<Request, Response>`, else the `ActionOverload`
						// adapting a typed handler or a `Sequence`
						let action = Action::<Request, Response>::new_async(
							async |cx: ActionContext<Request>| -> Result<Response> {
								cx.caller.call::<Request, Response>(cx.input).await
							},
						);
						(action, entity)
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

	/// A route declaring an [`HttpMethod`] only dispatches to that method, and a
	/// `Get` route also answers `Head`.
	///
	/// Regression guard: the declared method was display-only, so a `GET` on the
	/// POST-only analytics beacon reached the body parser and answered `500` with
	/// an `Internal Error` logged for every bot probe.
	#[beet_core::test]
	async fn declared_method_gates_dispatch() {
		async fn status(method: HttpMethod, declared: HttpMethod) -> StatusCode {
			router_world()
				.spawn((Router::with_defaults(), children![(
					route::exchange("beacon", EchoParams),
					declared
				)]))
				.exchange(Request::new(method, "beacon"))
				.await
				.status()
		}
		status(HttpMethod::Post, HttpMethod::Post).await.xpect_eq(StatusCode::OK);
		status(HttpMethod::Get, HttpMethod::Post)
			.await
			.xpect_eq(StatusCode::METHOD_NOT_ALLOWED);
		status(HttpMethod::Get, HttpMethod::Get).await.xpect_eq(StatusCode::OK);
		// a HEAD is a GET with the body dropped, so a Get route serves it
		status(HttpMethod::Head, HttpMethod::Get)
			.await
			.xpect_eq(StatusCode::OK);
	}

	#[beet_core::test]
	async fn dynamic_segment_reaches_handler() {
		router_world()
			.spawn((Router::with_defaults(), children![route::exchange(
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
			.spawn((Router::with_defaults(), children![route::exchange(
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
			.spawn((Router::with_defaults(), children![route::exchange(
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
			.spawn((Router::with_defaults(), children![
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
			.spawn((Router::with_defaults(), children![
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
			.spawn((Router::with_defaults(), children![
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
			.spawn((Router::with_defaults(), children![
				Increment::bundle(FieldRef::new("count")),
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
			.spawn((Router::with_defaults(), children![
				Increment::bundle(FieldRef::new("count")),
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
			.spawn((Router::with_defaults(), children![Increment::bundle(FieldRef::new(
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
			.spawn((Router::with_defaults(), children![
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
			.spawn((Router::with_defaults(), children![
				(
					render_action::fixed_func_route("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![Increment::bundle(FieldRef::new("count")),],
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
			.spawn((Router::with_defaults(), children![Increment::bundle(FieldRef::new(
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
			.spawn((Router::with_defaults(), children![
				(
					render_action::fixed_func_route("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![Increment::bundle(FieldRef::new("count")),],
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
	/// [`Response`] via [`Response::sse`] — no special router needed. `Response::sse`
	/// and [`SseBody`] live behind `beet_net/http` (pulled by `native`); the json
	/// payload behind `beet_net/json` (pulled by `json`), so the test needs both.
	#[cfg(all(feature = "json", feature = "native"))]
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
			Response::sse(bevy::tasks::futures_lite::stream::iter(
				(0..3).map(|index| Ok(SseBody::message(Tick { index }))),
			))
		}

		router_world()
			.spawn((Router::with_defaults(), children![route::exchange(
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
