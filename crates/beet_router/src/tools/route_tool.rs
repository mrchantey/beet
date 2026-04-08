use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_tool::prelude::*;

/// Create a routable tool that can be called with [`Request`]/[`Response`]
/// pairs. The request body is deserialized to `In` and the `Out` is
/// serialized back into the response body.
///
/// Unlike a bare tool, this constructor:
/// - Inserts a [`PathPartial`] from the provided `path`
/// - Wraps the inner typed tool with serde middleware via [`IntoWrapTool`]
///
/// Content-type negotiation is based on the request's `content-type` header:
/// - `application/json` (default): uses `serde_json`
/// - `application/x-postcard`: uses `postcard` binary serialization
///
/// The response `content-type` mirrors the request format.
///
/// ## Example
///
/// ```
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_tool::prelude::*;
///
/// // Create a route tool from a typed handler
/// let bundle = route_tool(
///     "add",
///     func_tool(|input: FuncToolIn<(i32, i32)>| Ok(input.0 + input.1)),
/// );
/// ```
pub fn route_tool<T: 'static, M>(
	path: impl AsRef<std::path::Path>,
	tool: T,
) -> impl Bundle
where
	T: IntoTool<M>,
	T::In: 'static + Send + Sync + serde::de::DeserializeOwned,
	T::Out: 'static + Send + Sync + serde::Serialize,
{
	(
		PathPartial::new(path),
		serde_exchange::<T::In, T::Out>.wrap(tool),
	)
}

/// Serde middleware that bridges [`Request`]/[`Response`] to typed
/// tool calls. Deserializes the request body, calls the inner handler
/// via [`Next`], serializes the output, and returns a [`Response`].
///
/// Errors propagate as tool errors through the handler chain.
async fn serde_exchange<Input, Output>(
	request: Request,
	next: Next<Input, Output>,
) -> Result<Response>
where
	Input: 'static + Send + Sync + serde::de::DeserializeOwned,
	Output: 'static + Send + Sync + serde::Serialize,
{
	let request_type = request
		.headers
		.get::<header::ContentType>()
		.and_then(|res| res.ok())
		.unwrap_or(MediaType::Json);
	let accepts = request
		.headers
		.get::<header::Accept>()
		.and_then(|res| res.ok())
		.unwrap_or_default();
	let body_bytes = request.body.into_bytes().await?;
	let input: Input = request_type.deserialize(&body_bytes)?;
	let output: Output = next.call(input).await?;

	let (response_type, response_body) =
		MediaType::serialize_accepts(&accepts, &output)?;

	Response::ok()
		.with_content_type(response_type)
		.with_body(response_body)
		.xok()
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_tool::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;

	#[derive(Debug, Serialize, Deserialize, Reflect, PartialEq)]
	struct AddInput {
		a: i32,
		b: i32,
	}

	fn add_route_tool() -> impl Bundle {
		route_tool(
			"add",
			func_tool(|input: FuncToolIn<AddInput>| Ok(input.a + input.b)),
		)
	}

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	// -- Request/Response JSON round-trip --

	#[beet_core::test]
	async fn json_request_response() {
		let request =
			Request::with_json("/add", &AddInput { a: 10, b: 20 }).unwrap();

		let response = AsyncPlugin::world()
			.spawn(add_route_tool())
			.call::<Request, Response>(request)
			.await
			.unwrap();

		response.status().xpect_eq(StatusCode::OK);
		response
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);
		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(30);
	}

	// -- JSON is the default when no content-type is specified --

	#[beet_core::test]
	async fn json_default_content_type() {
		let request = Request::post("/add").with_body(r#"{"a":1,"b":2}"#);

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call::<Request, Response>(request)
			.await
			.unwrap()
			.deserialize_blocking::<u32>()
			.unwrap()
			.xpect_eq(3);
	}

	// -- postcard binary round-trip --

	#[beet_core::test]
	#[cfg(feature = "postcard")]
	async fn postcard_request_response() {
		let request = Request::with_postcard("/add", &AddInput { a: 5, b: 7 })
			.unwrap()
			.with_header::<header::Accept>(MediaType::Postcard);

		let response = AsyncPlugin::world()
			.spawn(add_route_tool())
			.call::<Request, Response>(request)
			.await
			.unwrap();

		response
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Postcard);

		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(12);
	}

	// -- unit input with empty body --

	#[beet_core::test]
	async fn unit_input_empty_body() {
		let response = AsyncPlugin::world()
			.spawn(route_tool("unit", func_tool(|_: FuncToolIn<()>| Ok(42i32))))
			.call::<Request, Response>(Request::get("/"))
			.await
			.unwrap();

		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(42);
	}

	// -- Response::deserialize round-trip --

	#[beet_core::test]
	#[cfg(feature = "json")]
	async fn response_deserialize_json_roundtrip() {
		let request =
			Request::with_json("/add", &AddInput { a: 10, b: 5 }).unwrap();

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call::<Request, Response>(request)
			.await
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(15);
	}

	#[beet_core::test]
	#[cfg(feature = "postcard")]
	async fn response_deserialize_postcard_roundtrip() {
		let request =
			Request::with_postcard("/add", &AddInput { a: 3, b: 9 }).unwrap();

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call::<Request, Response>(request)
			.await
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(12);
	}

	// -- with_json_str convenience --

	#[beet_core::test]
	#[cfg(feature = "json")]
	async fn json_str_request() {
		let request = Request::with_json_str("/add", r#"{"a":100,"b":200}"#);

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call::<Request, Response>(request)
			.await
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(300);
	}

	// -- route tool inserts path partial --

	#[test]
	fn inserts_path_partial() {
		router_world()
			.spawn(add_route_tool())
			.get::<PathPattern>()
			.unwrap()
			.annotated_route_path()
			.to_string()
			.xpect_eq("/add");
	}
}
