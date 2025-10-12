use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde::de::DeserializeOwned;
use std::future::Future;


/// A route handler accepting an input type to be extracted from the request.
/// - For requests with no body, ie `GET`, the input is deserialized from the query parameters.
/// - For requests with a body, ie `POST`, `PUT`, etc, the input is deserialized from the body.
///
/// To support the [`JsonResult`] pattern, the handler must return a type that implements [`IntoResponse`],
/// regular types can be mapped by using `my_action.pipe(Json::pipe)`.
pub fn action_endpoint<T, Input, Out, M1, M2>(
	method: HttpMethod,
	handler: T,
) -> (HttpMethod, (Endpoint, RouteHandler))
where
	T: 'static + Send + Sync + Clone + IntoSystem<Input, Out, M1>,
	Input: 'static + SystemInput,
	for<'a> Input::Inner<'a>: DeserializeOwned,
	Out: 'static + Send + Sync + IntoResponse<M2>,
{
	let handler = match method.has_body() {
		// ie `POST`, `PUT`, etc
		true => RouteHandler::endpoint::<_, _, _, Out, _, _>(
			move |val: In<Json<Input::Inner<'_>>>,
			      world: &mut World|
			      -> Result<Out> {
				let out =
					world.run_system_cached_with(handler.clone(), val.0.0)?;
				Ok(out)
			},
		),
		// ie `GET`, `DELETE`, etc
		false => RouteHandler::endpoint::<_, _, _, Out, _, _>(
			move |val: In<JsonQueryParams<Input::Inner<'_>>>,
			      world: &mut World|
			      -> Result<Out> {
				let out =
					world.run_system_cached_with(handler.clone(), val.0.0)?;
				Ok(out)
			},
		),
	};
	(method, handler)
}
pub fn action_endpoint_async<T, Input, Fut, Out, M2>(
	method: HttpMethod,
	handler: T,
) -> (HttpMethod, RouteHandler)
where
	T: 'static + Send + Sync + Clone + Fn(In<Input>, World, Entity) -> Fut,
	Input: 'static + Send + Sync + DeserializeOwned,
	Out: 'static + Send + Sync + IntoResponse<M2>,
	Fut: 'static + Send + Future<Output = (World, Out)>,
{
	let handler = match method.has_body() {
		// ie `POST`, `PUT`, etc
		true => RouteHandler::new_async(
			async move |world: World, input: Json<Input>, entity: Entity| {
				let (world, out) = handler(In(input.0), world, entity).await;
				(world, out)
			},
		),
		// ie `GET`, `DELETE`, etc
		false => RouteHandler::new_async(
			async move |world: World,
			            input: JsonQueryParams<Input>,
			            entity: Entity| {
				let (world, out) = handler(In(input.0), world, entity).await;
				(world, out)
			},
		),
	};
	(method, handler)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn action() {
		Router::new_bundle(|| {
			action_endpoint(HttpMethod::Get, |In(input): In<u32>| {
				Json(format!("hello {}", input))
			})
		})
		.oneshot("/?data=42")
		.await
		.into_result()
		.await
		.unwrap()
		.json::<String>()
		.await
		.unwrap()
		.xpect_eq("hello 42");
	}
	#[sweet::test]
	async fn action_async() {
		Router::new_bundle(|| {
			action_endpoint_async(
				HttpMethod::Get,
				async move |In(input): In<u32>, _world, _entity| {
					(_world, Json(input))
				},
			)
		})
		.oneshot("/?data=42")
		.await
		.into_result()
		.await
		.unwrap()
		.json::<u32>()
		.await
		.unwrap()
		.xpect_eq(42);
	}
}
