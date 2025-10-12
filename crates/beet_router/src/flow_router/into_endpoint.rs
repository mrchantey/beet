use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;


/// A trait for defining endpoint functions.
pub trait IntoEndpointFn<M> {
	fn into_handler_fn(self) -> impl Bundle;
}


/// Take the request or insert a 500 response if not found
fn take_request_or_500(world: &mut World, entity: Entity) -> Option<Request> {
	if let Some(req) = world.entity_mut(entity).take::<Request>() {
		Some(req)
	} else {
		error!(
			"
No Request found for endpoint, this is usually because it has already
been taken by a previous route, please check for conficting endpoints.
			"
		);
		world
			.entity_mut(entity)
			.insert(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR));
		None
	}
}

pub struct AsyncWorldRequestIntoHandlerFn;
impl<Func, Fut, Req, Res, M1, M2>
	IntoEndpointFn<(Req, Res, M1, M2, AsyncWorldRequestIntoHandlerFn)> for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce(AsyncWorld, Req) -> Fut,
	Fut: Send + Future<Output = Res>,
	Req: Send + FromRequest<M1>,
	Res: IntoResponse<M2>,
{
	fn into_handler_fn(self) -> impl Bundle {
		OnSpawn::observe(move |ev: On<GetOutcome>, mut commands: Commands| {
			let exchange = ev.agent();
			let this = self.clone();
			commands.queue(move |world: &mut World| {
				let Some(req) = take_request_or_500(world, exchange) else {
					return;
				};
				world.run_async(async move |world: AsyncWorld| {
					match Req::from_request(req).await {
						Ok(req) => {
							let response = this.clone()(world.clone(), req)
								.await
								.into_response();
							world.entity(exchange).insert(response).await;
						}
						Err(response) => {
							world.entity(exchange).insert(response).await;
						}
					};
				});
			});
		})
	}
}
