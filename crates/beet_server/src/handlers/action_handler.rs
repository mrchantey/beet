use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use serde::de::DeserializeOwned;
use std::future::Future;


impl RouteHandler {
	/// A route handler accepting an input type to be extracted from the request.
	/// - For requests with no body, ie `GET`, the input is deserialized from the query parameters.
	/// - For requests with a body, ie `POST`, `PUT`, etc, the input is deserialized from the body.
	///
	/// To support the [`JsonResult`] pattern, the handler must return a type that implements [`IntoResponse`],
	/// regular types can be mapped by using `my_action.pipe(Json::pipe)`.
	pub fn action<T, Input, Out, Marker>(
		endpoint: impl Into<Endpoint>,
		handler: T,
	) -> (Endpoint, Self)
	where
		T: 'static + Send + Sync + Clone + IntoSystem<Input, Out, Marker>,
		Input: 'static + SystemInput,
		for<'a> Input::Inner<'a>: DeserializeOwned,
		Out: 'static + Send + Sync + IntoResponse,
	{
		let endpoint = endpoint.into();
		match endpoint.method().has_body() {
			// ie `POST`, `PUT`, etc
			true => Self::new(
				endpoint,
				move |val: In<Json<Input::Inner<'_>>>,
				      world: &mut World|
				      -> Result<Out> {
					let out = world
						.run_system_cached_with(handler.clone(), val.0.0)?;
					Ok(out)
				},
			),
			// ie `GET`, `DELETE`, etc
			false => Self::new(
				endpoint,
				move |val: In<JsonQueryParams<Input::Inner<'_>>>,
				      world: &mut World|
				      -> Result<Out> {
					let out = world
						.run_system_cached_with(handler.clone(), val.0.0)?;
					Ok(out)
				},
			),
		}
	}
	pub fn action_async<T, Input, Fut, Out>(
		endpoint: impl Into<Endpoint>,
		handler: T,
	) -> (Endpoint, Self)
	where
		T: 'static + Send + Sync + Clone + Fn(In<Input>, World, Entity) -> Fut,
		Input: 'static + Send + Sync + DeserializeOwned,
		Out: 'static + Send + Sync + IntoResponse,
		Fut: 'static + Send + Future<Output = (World, Out)>,
	{
		let endpoint = endpoint.into();
		match endpoint.method().has_body() {
			// ie `POST`, `PUT`, etc
			true => (
				endpoint,
				Self::new_async(
					async move |world: World,
					            input: Json<Input>,
					            entity: Entity| {
						let (world, out) =
							handler(In(input.0), world, entity).await;
						(world, out)
					},
				),
			),
			// ie `GET`, `DELETE`, etc
			false => (
				endpoint,
				Self::new_async(
					async move |world: World,
					            input: JsonQueryParams<Input>,
					            entity: Entity| {
						let (world, out) =
							handler(In(input.0), world, entity).await;
						(world, out)
					},
				),
			),
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
	struct Foo {
		value: u32,
	}

	#[sweet::test]
	async fn action() {
		Router::new_bundle(|| {
			RouteHandler::action(HttpMethod::Get, |In(input): In<u32>| {
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
		.xpect()
		.to_be("hello 42");
	}
	#[sweet::test]
	async fn action_async() {
		Router::new_bundle(|| {
			RouteHandler::action_async(
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
		.xpect()
		.to_be(42);
	}
}
