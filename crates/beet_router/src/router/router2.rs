use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::IntoResponse;
use beet_net::prelude::*;
use beet_tool::prelude::*;

/// Routes a request to the matching tool in the [`RouteTree`],
/// applying ancestor [`MiddlewareList`] around the matched tool.
///
/// When no route matches, renders contextual not-found help.
/// Middleware such as [`HelpHandler`] and [`NavigateHandler`] wrap
/// the inner tool so they can intercept before dispatch.
#[tool]
#[derive(Debug, Clone, Component)]
pub async fn Router2(cx: ToolContext<Request>) -> Response {
	let caller = cx.caller.clone();
	let world = cx.world();
	let request = cx.input;
	let path = request.path().clone();

	let node = world
		.with_state::<AncestorQuery<&RouteTree>, _>(move |query| {
			query.get(caller.id()).map(|tree| tree.find(&path).cloned())
		})
		.await;

	let inner_tool = match &node {
		Ok(Some(node)) => {
			let entity = world.entity(node.entity);
			match entity.clone().get_cloned::<ExchangeTool>().await {
				Ok(tool) => tool.into_tool(),
				Err(err) => return err.into_response(),
			}
		}
		Ok(None) => {
			// No matching route — build a not-found response through
			// the contextual help system so middleware still applies.
			ContextualNotFound.into_tool()
		}
		Err(err) => return bevyhow!("{err}").into_response(),
	};

	// Collect middleware from ancestors and wrap the inner tool
	let caller_id = cx.caller.id();
	let tool = world
		.with_state::<AncestorQuery<&MiddlewareList<Request, Response>>, _>(
			move |query| {
				let mut wrapped = inner_tool;
				for list in query.get_ancestors(caller_id) {
					wrapped = list.wrap(&wrapped);
				}
				wrapped
			},
		)
		.await;

	cx.caller
		.call_detached(tool, request)
		.await
		.unwrap_or_else(|err| err.into_response())
}


/// Type-erased `Tool<Request, Response>` stored on each route entity.
/// Handles request extraction and response conversion so the router
/// can dispatch uniformly.
///
/// Inserts [`ToolMeta`] on add so that route-building observers
/// (which trigger on `Insert<ToolMeta>`) fire correctly.
#[derive(Clone, Component)]
#[component(on_add = on_add_exchange_tool)]
pub struct ExchangeTool {
	inner: Tool<Request, Response>,
}

fn on_add_exchange_tool(mut world: DeferredWorld, cx: HookContext) {
	let meta = ToolMeta::of::<ExchangeTool, Request, Response>();
	world.commands().entity(cx.entity).insert(meta);
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

// ///None = not found?
// impl<T: IntoResponse<M>, M> IntoResponse<(Self, M)> for Option<T> {
// 	fn into_response(self) -> Response {
// 		match self {
// 			Some(val) => val.into_response(),
// 			None => Response::not_found(),
// 		}
// 	}
// }



/// Marker component for scene entities that should be despawned
/// after they render.
#[derive(Component)]
pub struct DespawnOnRender;

/// A route that returns an [`Entity`] is a scene route.
/// The entity is rendered via the ancestor [`SceneToolRenderer`]
/// and then converted to a response. Entities marked with
/// [`DespawnOnRender`] are cleaned up after rendering.
impl ExchangeRouteOut<Self> for Entity {
	fn into_route_response(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move {
			SceneToolRenderer::render_entity(&caller, self, parts).await
		})
	}
}

/// Convenience function to create a simple route from a path and bundle.
pub fn route<B: Bundle>(path: &str, bundle: B) -> (PathPartial, B) {
	(PathPartial::new(path), bundle)
}
