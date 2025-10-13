use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde::de::DeserializeOwned;




/// An [`Endpoint`] wrapper accepting an input type to be extracted from the request.
/// - For requests with no body, ie `GET`, the input is deserialized from the query parameters via [`JsonQueryParams`].
/// - For requests with a body, ie `POST`, `PUT`, etc, the input is deserialized from the body via [`Json`].
///
/// Note that actions without an input must receive a request with a unit type json.
///
/// To support flexibility in the return type, including the [`JsonResult`] pattern,
/// the handler must return a type that implements [`IntoResponse`],
/// regular Serde types can be mapped by using `my_action.pipe(Json::pipe)`.
pub struct ServerAction;


impl ServerAction {
	pub fn build<T, Input, Out, M1, M2>(
		method: HttpMethod,
		handler: T,
	) -> EndpointBuilder
	where
		T: 'static + Send + Sync + Clone + IntoSystem<Input, Out, M1>,
		Input: 'static + Send + SystemInput,
		for<'a> Input::Inner<'a>: 'static + Send + Sync + DeserializeOwned,
		Out: 'static + Send + Sync + IntoResponse<M2>,
	{
		let builder = Endpoint::builder().with_method(method);
		match method.has_body() {
			// ie `POST`, `PUT`, etc
			true => builder.with_handler(
				async move |entity: AsyncEntity,
				            req: Json<Input::Inner<'_>>|
				            -> Result<Out> {
					let out = entity
						.world()
						.run_system_cached_with(handler.clone(), req.0)
						.await?;
					Ok(out)
				},
			),
			// ie `GET`, `DELETE`, etc
			false => builder.with_handler(
				async move |entity: AsyncEntity,
				            req: JsonQueryParams<Input::Inner<'_>>|
				            -> Result<Out> {
					let out = entity
						.world()
						.run_system_cached_with(handler.clone(), req.0)
						.await?;
					Ok(out)
				},
			),
		}
	}

	pub fn build_async<T, Input, Fut, Out, M2>(
		method: HttpMethod,
		handler: T,
	) -> EndpointBuilder
	where
		T: 'static + Send + Sync + Clone + Fn(In<Input>, AsyncEntity) -> Fut,
		Input: 'static + Send + Sync + DeserializeOwned,
		Out: 'static + Send + Sync + IntoResponse<M2>,
		Fut: 'static + Send + Future<Output = Out>,
	{
		let builder = Endpoint::builder().with_method(method);
		match method.has_body() {
			// ie `POST`, `PUT`, etc
			true => builder.with_handler(
				async move |entity: AsyncEntity, req: Json<Input>| -> Out {
					handler.clone()(In(req.0), entity).await
				},
			),
			// ie `GET`, `DELETE`, etc
			false => builder.with_handler(
				async move |entity: AsyncEntity,
				            req: JsonQueryParams<Input>|
				            -> Out {
					handler.clone()(In(req.0), entity).await
				},
			),
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn no_input() {
		FlowRouterPlugin::world()
			.spawn((
				RouteServer,
				ServerAction::build(HttpMethod::Post, (|| 2).pipe(Json::pipe)),
			))
			.oneshot(
				Request::post("/")
					// no input means we need to specify unit type
					.with_json_body(&())
					.unwrap(),
			)
			.await
			.into_result()
			.await
			.unwrap()
			.body
			.into_json::<u32>()
			.await
			.unwrap()
			.xpect_eq(2);
	}
	#[sweet::test]
	async fn post() {
		let mut world = FlowRouterPlugin::world();
		let mut entity = world.spawn((
			RouteServer,
			ServerAction::build(
				HttpMethod::Post,
				(|val: In<u32>| val.0 + 2).pipe(Json::pipe),
			)
			.with_path("foo"),
		));

		//ok
		entity
			.oneshot(Request::post("/foo").with_json_body(&3).unwrap())
			.await
			.into_result()
			.await
			.unwrap()
			.body
			.into_json::<u32>()
			.await
			.unwrap()
			.xpect_eq(5);
		// no body
		entity
			.oneshot(Request::post("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::BAD_REQUEST);
	}
	#[sweet::test]
	async fn get_sync() {
		let mut world = FlowRouterPlugin::world();
		let mut entity = world.spawn((
			RouteServer,
			ServerAction::build_async(
				HttpMethod::Get,
				async |val: In<u32>, _| Json(val.0 + 2),
			),
		));

		//ok
		entity
			.oneshot(Request::get("/?data=3"))
			.await
			.into_result()
			.await
			.unwrap()
			.body
			.into_json::<u32>()
			.await
			.unwrap()
			.xpect_eq(5);
		// no query param
		entity
			.oneshot(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::BAD_REQUEST);
	}
}
