use std::sync::Arc;

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::IntoResponse;
use beet_net::prelude::*;
use beet_tool::prelude::*;

#[tool]
#[derive(Debug, Clone, Component)]
pub async fn Router2(cx: ToolContext<Request>) -> Response {
	let caller = cx.caller.id();
	let world = cx.world();
	let request = cx.input;
	let path = request.path().clone();

	let node = world
		.with_state::<AncestorQuery<&RouteTree>, _>(move |query| {
			query.get(caller).map(|tree| tree.find(&path).cloned())
		})
		.await;


	match node {
		Ok(Some(tool_node)) => {
			let entity = world.entity(tool_node.entity);
			let tool = match entity.clone().get_cloned::<ExchangeTool>().await {
				Ok(tool) => tool,
				Err(err) => return err.into_response(),
			};
			tool.call(entity, request)
				.await
				.unwrap_or_else(|err| err.into_response())
		}
		Ok(None) => bevyhow!("fd").into_response(),
		Err(err) => err.into_response(),
	}
}


#[derive(Clone, Component)]
pub struct ExchangeTool {
	handler: Arc<
		dyn 'static
			+ Send
			+ Sync
			+ Fn(
				AsyncEntity,
				Request,
			) -> MaybeSendBoxedFuture<'static, Result<Response>>,
	>,
}

impl ExchangeTool {
	pub fn new<In, Out, M1, M2>() -> Self
	where
		In: 'static + Send + Sync + FromRequest<M1>,
		Out: 'static + Send + Sync + ExchangeRouteOut<M2>,
	{
		Self {
			handler: Arc::new(|entity, request| {
				Box::pin(async move {
					let parts = request.parts().clone();
					let input = In::from_request(request).await?;
					let output: Out = entity.call(input).await?;
					output.into_route_response(entity, parts).await
				})
			}),
		}
	}

	pub fn call(
		&self,
		entity: AsyncEntity,
		request: Request,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		(self.handler)(entity, request)
	}
}


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

struct SerdeIntoResponseMarker;
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

/// A route that return an entity is a Scene Route.
/// This indicates the returned entity should be rendered
/// and then converted to a response.
///
/// For static routes the returned entity is usually the caller,
/// but caching strategies can determine any entity for rendering.
impl ExchangeRouteOut<Self> for Entity {
	fn into_route_response(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move {
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
				.unwrap_or_else(|_| async_tool(default_scene_renderer));
			caller
				.world()
				.entity(self)
				.call_detached(render_tool, parts)
				.await
		})
	}
}
