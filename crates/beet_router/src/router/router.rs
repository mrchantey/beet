use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::IntoResponse;
use beet_net::prelude::*;
use beet_tool::prelude::*;

/// Creates a router bundle with help and navigate middleware.
///
/// This is the standard way to set up routing. It includes:
/// - [`Router2`] for route lookup and dispatch
/// - [`HelpHandler`] middleware for `--help` support
/// - [`NavigateHandler`] middleware for `--navigate` support
///
/// Does **not** include a [`SceneToolRenderer`] — rendering falls
/// back to the default renderer when no [`SceneToolRenderer`] is
/// found on an ancestor.
pub fn router() -> impl Bundle {
	(
		Router,
		Middleware::<HelpHandler, Request, Response>::default(),
		Middleware::<NavigateHandler, Request, Response>::default(),
	)
}

/// Routes a request to the matching tool in the [`RouteTree`],
/// applying ancestor [`MiddlewareList`] around the matched tool.
///
/// When no route matches, renders contextual not-found help.
/// Middleware such as [`HelpHandler`] and [`NavigateHandler`] wrap
/// the inner tool so they can intercept before dispatch.
#[tool]
#[derive(Debug, Clone, Component)]
pub async fn Router(cx: ToolContext<Request>) -> Response {
	let caller = cx.caller.clone();
	let world = cx.world();
	let request = cx.input;
	let path = request.path().clone();

	// find the matching route in the tree
	let node = world
		.with_state::<AncestorQuery<&RouteTree>, _>(move |query| {
			query.get(caller.id()).map(|tree| tree.find(&path).cloned()).map_err(|_|{
				bevyhow!("Route tree not found. Was the `ToolMeta` added? was the `RouterPlugin` added?")
			})
		})
		.await;

	// resolve the inner tool and dispatch entity from the matched route
	let (inner_tool, dispatch_entity) = match &node {
		Ok(Some(node)) => {
			let entity = world.entity(node.entity);
			match entity.clone().get_cloned::<ExchangeTool>().await {
				Ok(tool) => (tool.into_tool(), entity),
				Err(err) => return err.into_response(),
			}
		}
		Ok(None) => {
			// no matching route — build a not-found response through
			// the contextual help system so middleware still applies
			(ContextualNotFound.into_tool(), cx.caller.clone())
		}
		Err(err) => return bevyhow!("{err}").into_response(),
	};

	// wrap the inner tool with ancestor middleware resolved from the
	// dispatch entity so route-scoped middleware is correctly applied
	let dispatch_id = dispatch_entity.id();
	let tool = world
		.with_state::<MiddlewareQuery, _>(move |query| {
			query.resolve_tool(dispatch_id, inner_tool)
		})
		.await;

	// dispatch on the route entity so cx.caller in the exchange tool
	// (and inner tools) is the route entity, not the server entity
	dispatch_entity
		.call_detached(tool, request)
		.await
		.unwrap_or_else(|err| err.into_response())
}


/// Type-erased `Tool<Request, Response>` stored on each route entity.
/// Handles request extraction and response conversion so the router
/// can dispatch uniformly.
#[derive(Clone, Component)]
pub struct ExchangeTool {
	inner: Tool<Request, Response>,
}

impl ExchangeTool {
	/// Creates an exchange tool that calls the entity's own typed tool,
	/// extracting input from the request and converting output to a response.
	pub fn new<In, Out, M1, M2>() -> Self
	where
		In: 'static + Send + Sync + FromRequest<M1>,
		Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
	{
		Self {
			inner: Tool::new_async(
				async |cx: ToolContext<Request>| -> Result<Response> {
					let parts = cx.input.parts().clone();
					let input = In::from_request(cx.input).await?;
					let output: Out = cx.caller.call(input).await?;
					output.into_route_response(cx.caller, parts).await
				},
			),
		}
	}

	/// Wraps an existing `Tool<Request, Response>` directly.
	/// Use this when you already have a fully constructed tool
	/// that handles its own request/response lifecycle.
	pub fn from_tool(tool: Tool<Request, Response>) -> Self {
		Self { inner: tool }
	}

	/// Creates an exchange tool that calls a detached inner tool
	/// instead of the entity's own tool.
	pub fn new_detached<In, Out, Inner, M1, M2, M3>(inner: Inner) -> Self
	where
		In: 'static + Send + Sync + FromRequest<M1>,
		Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
		Inner: 'static + Send + Sync + IntoTool<M3, In = In, Out = Out>,
	{
		let inner = inner.into_tool();
		Self {
			inner: Tool::new_async(
				async move |cx: ToolContext<Request>| -> Result<Response> {
					let parts = cx.input.parts().clone();
					let input = In::from_request(cx.input).await?;
					let output: Out =
						cx.caller.call_detached(inner, input).await?;
					output.into_route_response(cx.caller, parts).await
				},
			),
		}
	}

	/// Dispatches a request through this exchange tool on the given entity.
	pub async fn call(
		&self,
		entity: AsyncEntity,
		request: Request,
	) -> Result<Response> {
		entity.call_detached(self.inner.clone(), request).await
	}
}

impl IntoTool<Self> for ExchangeTool {
	type In = Request;
	type Out = Response;
	fn into_tool(self) -> Tool<Request, Response> { self.inner }
}

/// Trait for converting a tool's output into a [`Response`].
///
/// Three blanket impls cover the main cases:
/// - Types implementing [`IntoResponse`] (direct conversion)
/// - Types implementing [`Serialize`] (serde content negotiation)
/// - [`Entity`] (scene rendering via [`SceneToolRenderer`])
pub trait ExchangeRouteOut<M>
where
	Self: Sized,
{
	fn into_route_response(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>>;
}


impl<T, M> ExchangeRouteOut<(T, M)> for T
where
	T: IntoResponse<M>,
{
	fn into_route_response(
		self,
		_caller: AsyncEntity,
		_parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		let response = self.into_response();
		Box::pin(async move { response.xok() })
	}
}

/// Marker type for the [`Serialize`] blanket impl of [`ExchangeRouteOut`].
pub struct SerdeIntoResponseMarker;
impl<T> ExchangeRouteOut<SerdeIntoResponseMarker> for T
where
	T: 'static + Send + Sync + Serialize,
{
	fn into_route_response(
		self,
		_caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move {
			let accept = match parts.headers.get_or_default::<header::Accept>()
			{
				Ok(accept) => accept,
				Err(err) => {
					return HttpError::new(
						StatusCode::BAD_REQUEST,
						format!("failed to parse accept headers: {}", err),
					)
					.into_response()
					.xok();
				}
			};
			let bytes = MediaType::serialize_accepts(&accept, &self)?;
			Response::ok().with_media(bytes).xok()
		})
	}
}

/// A route that returns an [`Entity`] is a scene route.
/// The entity is rendered via the ancestor [`SceneToolRenderer`]
/// and then converted to a response. Entities marked with
/// [`DespawnOnRender`] are cleaned up after rendering.
impl ExchangeRouteOut<Self> for SceneEntity {
	fn into_route_response(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move {
			SceneToolRenderer::render_entity(&caller, self.0, parts).await
		})
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deref)]
pub struct SceneEntity(Entity);
impl SceneEntity {
	/// A scene entity that should not be despawned after render
	pub fn new_fixed(entity: Entity) -> Self { Self(entity) }

	/// A scene entity that should be despawned after render,
	/// ie a help page or not found route
	pub fn new_ephemeral(_entity: Entity) -> Self { todo!() }
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_node::prelude::*;
	use beet_tool::prelude::*;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[beet_core::test]
	async fn route_renders_scene() {
		router_world()
			.spawn((router(), children![fixed_scene("about", || {
				Element::new("p").with_inner_text("About page")
			}),]))
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
			.spawn((router(), children![fixed_scene("", || {
				Element::new("p").with_inner_text("Root content")
			}),]))
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
				fixed_scene("", || {
					children![
						Element::new("h1").with_inner_text("My Server"),
						Element::new("p").with_inner_text("welcome!"),
					]
				}),
				fixed_scene("about", || {
					Element::new("p").with_inner_text("about")
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
	async fn help_flag_returns_route_list() {
		router_world()
			.spawn((router(), children![
				increment(FieldRef::new("count")),
				fixed_scene("about", || {
					Element::new("p").with_inner_text("about")
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
			.spawn((router(), children![
				increment(FieldRef::new("count")),
				fixed_scene("about", || {
					Element::new("p").with_inner_text("about")
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
			.spawn((router(), children![increment(FieldRef::new("count")),]))
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
			.spawn((router(), children![
				fixed_scene("", || {
					children![
						Element::new("h1").with_inner_text("My Server"),
						Element::new("p").with_inner_text("welcome!"),
					]
				}),
				fixed_scene("about", || {
					Element::new("p").with_inner_text("about")
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
			.spawn((router(), children![
				(
					fixed_scene("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![increment(FieldRef::new("count")),],
				),
				fixed_scene("about", || {
					Element::new("p").with_inner_text("about")
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

	#[beet_core::test]
	async fn not_found_shows_ancestor_help() {
		router_world()
			.spawn((router(), children![increment(FieldRef::new("count")),]))
			.call::<Request, Response>(
				Request::from_cli_str("nonexistent").unwrap(),
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
					fixed_scene("counter", || {
						Element::new("p").with_inner_text("counter")
					}),
					children![increment(FieldRef::new("count")),],
				),
				fixed_scene("about", || {
					Element::new("p").with_inner_text("about")
				}),
			]))
			.call::<Request, Response>(
				Request::from_cli_str("counter nonsense").unwrap(),
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
}
