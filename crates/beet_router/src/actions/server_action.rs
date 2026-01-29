use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde::Serialize;
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


pub trait IntoServerActionOut<M> {
	fn into_action_response(self) -> Response;
}
pub struct SerdeResultIntoServerActionOut;
impl<T, E> IntoServerActionOut<(SerdeResultIntoServerActionOut, E)>
	for Result<T, E>
where
	T: Serialize,
	E: Serialize,
{
	fn into_action_response(self) -> Response {
		JsonResult::new(self).into_response()
	}
}
pub struct BevyResultIntoServerActionOut;
impl<T> IntoServerActionOut<Self> for Result<T, BevyError>
where
	T: Serialize,
{
	fn into_action_response(self) -> Response {
		self.map(|val| {
			serde_json::to_string(&val)
				.map(|val| Response::ok_body(val, "application/json"))
				.unwrap_or_else(|_| {
					Response::from_status_body(
						StatusCode::InternalError,
						"Failed to serialize response body",
						"text/plain",
					)
				})
				.into_response()
		})
		.into_response()
	}
}
pub struct TypeIntoServerActionOut;
impl<T> IntoServerActionOut<(TypeIntoServerActionOut,)> for T
where
	T: Serialize,
{
	fn into_action_response(self) -> Response { Json(self).into_response() }
}



impl ServerAction {
	pub fn new<T, Input, Out, M1, M2>(
		method: HttpMethod,
		handler: T,
	) -> EndpointBuilder
	where
		T: 'static + Send + Sync + Clone + IntoSystem<Input, Out, M1>,
		Input: 'static + Send + SystemInput,
		for<'a> Input::Inner<'a>: 'static + Send + Sync + DeserializeOwned,
		Out: 'static + Send + Sync + IntoServerActionOut<M2>,
	{
		let builder = EndpointBuilder::default().with_method(method);
		match method.has_body() {
			// ie `POST`, `PUT`, etc
			true => builder.with_action(
				async move |req: Json<Input::Inner<'_>>,
				            action: AsyncEntity|
				            -> Result<Response> {
					let out = action
						.world()
						.run_system_cached_with(handler.clone(), req.0)
						.await?;
					Ok(out.into_action_response())
				},
			),
			// ie `GET`, `DELETE`, etc
			false => builder.with_action(
				async move |req: JsonQueryParams<Input::Inner<'_>>,
				            action: AsyncEntity|
				            -> Result<Response> {
					let out = action
						.world()
						.run_system_cached_with(handler.clone(), req.0)
						.await?;
					Ok(out.into_action_response())
				},
			),
		}
	}

	pub fn new_async<T, Input, Fut, Out, M2>(
		method: HttpMethod,
		handler: T,
	) -> EndpointBuilder
	where
		T: 'static + Send + Sync + Clone + Fn(Input, AsyncEntity) -> Fut,
		Input: 'static + Send + Sync + DeserializeOwned,
		Out: 'static + Send + Sync + IntoServerActionOut<M2>,
		Fut: 'static + Send + Future<Output = Out>,
	{
		let builder = EndpointBuilder::default().with_method(method);
		match method.has_body() {
			// ie `POST`, `PUT`, etc
			true => builder.with_action(
				async move |req: Json<Input>, action: AsyncEntity| {
					handler.clone()(req.0, action).await.into_action_response()
				},
			),
			// ie `GET`, `DELETE`, etc
			false => builder.with_action(
				async move |req: JsonQueryParams<Input>,
				            action: AsyncEntity| {
					handler.clone()(req.0, action).await.into_action_response()
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

	#[beet_core::test]
	async fn no_input() {
		RouterPlugin::world()
			.spawn(flow_exchange(|| ServerAction::new(HttpMethod::Post, || 2)))
			.exchange(
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
	#[beet_core::test]
	async fn post() {
		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(flow_exchange(|| {
			ServerAction::new(HttpMethod::Post, |val: In<u32>| val.0 + 2)
				.with_path("foo")
		}));

		//ok
		entity
			.exchange(Request::post("/foo").with_json_body(&3).unwrap())
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
			.exchange(Request::post("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::MalformedRequest);
	}
	#[beet_core::test]
	async fn get_sync() {
		let mut world = RouterPlugin::world();
		let mut entity = world.spawn(flow_exchange(|| {
			ServerAction::new_async(HttpMethod::Get, async |val: u32, _| {
				val + 2
			})
		}));

		//ok
		entity
			.exchange(Request::get("/?data=3"))
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
			.exchange(Request::get("/"))
			.await
			.status()
			.xpect_eq(StatusCode::MalformedRequest);
	}
}
