use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

/// An `action` / `exchange` pair for a current visit.
#[derive(Clone)]
pub struct VisitContext {
	/// The current action this exchange is visiting
	action: Entity,
	/// The `agent` of the action, containing the [`Request`] and [`Response`]
	exchange: Entity,
	/// The world the action is running in
	pub world: AsyncWorld,
}

impl std::ops::Deref for VisitContext {
	type Target = AsyncWorld;
	fn deref(&self) -> &Self::Target { &self.world }
}

impl VisitContext {
	pub fn action_id(&self) -> Entity { self.action }
	pub fn exchange_id(&self) -> Entity { self.exchange }
	/// The action entity this endpoint currently running for
	pub fn action(&self) -> AsyncEntity { self.world.entity(self.action) }
	/// The exchange entity, containing the [`Request`] and [`Response`]
	pub fn exchange(&self) -> AsyncEntity { self.world.entity(self.exchange) }
	/// The world this endpoint is running in
	pub fn world(&self) -> &AsyncWorld { &self.world }
}

/// Helper for defining methods accepting requests and returning responses.
/// These are converted to `On<GetOutcome>` observers.
/// In the case where a request cannot be found a 500 response is inserted.
pub trait IntoEndpoint<M> {
	fn into_endpoint(self) -> impl Bundle;
}

fn into_endpoint_inner<Req, Res, Func, Fut, M1, M2>(func: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + FnOnce(Req, VisitContext) -> Fut,
	Fut: Send + Future<Output = Res>,
	Req: Send + FromRequest<M1>,
	Res: IntoResponse<M2>,
{
	OnSpawn::observe(move |ev: On<GetOutcome>, mut commands: Commands| {
		let func = func.clone();
		let action = ev.action();
		let exchange = ev.agent();
		commands.queue(move |world: &mut World| {
			// try to tak the request
			match world.entity_mut(exchange).take::<Request>() {
				Some(req) => {
					world.run_async(async move |world: AsyncWorld| {
						match Req::from_request(req).await {
							Ok(req) => {
								let context = VisitContext {
									action,
									exchange,
									world: world.clone(),
								};

								let res = func(req, context).await;
								let response = res.into_response();
								world.entity(exchange).insert(response).await;
								// only pass condition
								world
									.entity(action)
									.trigger_target(
										Outcome::Pass.with_agent(exchange),
									)
									.await;
							}
							Err(response) => {
								world.entity(exchange).insert(response).await;
								world
									.entity(action)
									.trigger_target(
										Outcome::Fail.with_agent(exchange),
									)
									.await;
							}
						}
					});
				}
				None => {
					error!(
						"
No Request found for endpoint, this is usually because it has already
been taken by a previous route, please check for conficting endpoints.
					"
					);
					world.entity_mut(exchange).insert(Response::from_status(
						StatusCode::INTERNAL_SERVER_ERROR,
					));
					world
						.entity_mut(action)
						.trigger_target(Outcome::Fail.with_agent(exchange));
				}
			}
		});
	})
}

pub struct ResponseIntoEndpoint;
impl<T, M> IntoEndpoint<(ResponseIntoEndpoint, M)> for T
where
	T: 'static + Send + Sync + Clone + IntoResponse<M>,
{
	fn into_endpoint(self) -> impl Bundle {
		// skip all the async shenannigans, just insert the response
		OnSpawn::observe(
			move |mut ev: On<GetOutcome>, mut commands: Commands| {
				commands
					.entity(ev.agent())
					.insert(self.clone().into_response());
				ev.trigger_next(Outcome::Pass);
			},
		)
	}
}


pub struct SystemIntoEndpoint;
impl<System, Req, Out, M1, M2, M3>
	IntoEndpoint<(SystemIntoEndpoint, Req, Out, M1, M2, M3)> for System
where
	System: 'static + Send + Sync + Clone + IntoSystem<Req, Out, M1>,
	Req: 'static + Send + SystemInput,
	for<'a> Req::Inner<'a>: 'static + Send + Sync + FromRequest<M2>,
	Out: 'static + Send + Sync + IntoResponse<M3>,
{
	fn into_endpoint(self) -> impl Bundle {
		into_endpoint_inner(async move |req, cx| {
			cx.run_system_cached_with(self.clone(), req)
				.await
				.map_err(HttpError::from)
		})
	}
}


pub struct AsyncSystemIntoEndpoint;
impl<Func, Fut, Req, Res, M1, M2>
	IntoEndpoint<(AsyncSystemIntoEndpoint, Req, Res, M1, M2)> for Func
where
	Func: 'static + Send + Sync + Clone + FnOnce(Req, VisitContext) -> Fut,
	Fut: Send + Future<Output = Res>,
	Req: Send + FromRequest<M1>,
	Res: IntoResponse<M2>,
{
	fn into_endpoint(self) -> impl Bundle { into_endpoint_inner(self) }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use sweet::prelude::*;

	#[derive(Serialize, Deserialize)]
	struct Foo(u32);

	async fn assert<M>(endpoint: impl IntoEndpoint<M>) -> StatusCode {
		let mut world = FlowRouterPlugin::world();
		let exchange = world
			.spawn(Request::get("/foo").with_json_body(&Foo(3)).unwrap())
			.id();
		world
			.spawn(endpoint.into_endpoint())
			.trigger_target(GetOutcome.with_agent(exchange))
			.flush();
		AsyncRunner::flush_async_tasks(&mut world).await;
		world.entity(exchange).get::<Response>().unwrap().status()
	}

	#[sweet::test]
	async fn response() { assert(StatusCode::OK).await.xpect_eq(200); }



	#[sweet::test]
	async fn system() {
		fn my_async_system(_: In<Json<Foo>>) -> StatusCode { StatusCode::OK }
		assert(my_async_system).await.xpect_eq(200);
		assert(|_: In<Json<Foo>>| StatusCode::OK)
			.await
			.xpect_eq(200);
		assert(|| StatusCode::OK).await.xpect_eq(200);
	}
	#[sweet::test]
	async fn async_system() {
		async fn my_async_system(
			_req: Json<Foo>,
			_cx: VisitContext,
		) -> StatusCode {
			StatusCode::OK
		}
		assert(my_async_system).await.xpect_eq(200);
		assert(async |_: Json<Foo>, _: VisitContext| StatusCode::OK)
			.await
			.xpect_eq(200);
	}
}
