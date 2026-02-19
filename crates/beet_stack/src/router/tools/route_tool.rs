use crate::prelude::*;
use beet_core::prelude::*;

/// Create a routable tool that can be called with [`Request`]/[`Response`]
/// pairs. The request body is deserialized to `In` and the `Out` is
/// serialized back into the response body.
///
/// Unlike [`tool`], this constructor:
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
/// # use beet_stack::prelude::*;
/// # use beet_core::prelude::*;
///
/// // Create a route tool from a typed handler
/// let bundle = route_tool(
///     "add",
///     func_tool(|input: FuncToolIn<(i32, i32)>| Ok(input.0 + input.1)),
/// );
///
/// // Can be called with Request/Response
/// let request = Request::with_json("/add", &(2i32, 2i32)).unwrap();
/// let response = AsyncPlugin::world()
///     .spawn(bundle)
///     .call_blocking::<Request, Response>(request)
///     .unwrap();
///
/// let result: i32 = response.deserialize_blocking().unwrap();
/// assert_eq!(result, 4);
/// ```
///
/// ## Using with serialized request/response
///
/// Route tools support serialized request/response calls,
/// using the `content-type` header for format negotiation:
///
/// ```
/// # use beet_stack::prelude::*;
/// # use beet_core::prelude::*;
/// # use serde::{Serialize, Deserialize};
/// #[derive(Debug, Serialize, Deserialize, Reflect)]
/// struct AddInput { a: i32, b: i32 }
///
/// let request = Request::with_json("/", &AddInput { a: 10, b: 20 }).unwrap();
/// let response = AsyncPlugin::world()
///     .spawn(route_tool(
///         "add",
///         func_tool(|input: FuncToolIn<AddInput>| Ok(input.a + input.b)),
///     ))
///     .call_blocking::<Request, Response>(request)
///     .unwrap();
///
/// let result: i32 = response.deserialize_blocking().unwrap();
/// assert_eq!(result, 30);
/// ```
pub fn route_tool<H: 'static, M>(
	path: impl AsRef<std::path::Path>,
	handler: H,
) -> impl Bundle
where
	H: IntoToolHandler<M>,
	H::In: 'static + Send + Sync + serde::de::DeserializeOwned,
	H::Out: 'static + Send + Sync + serde::Serialize,
{
	(
		PathPartial::new(path),
		serde_exchange::<H::In, H::Out>.wrap(handler),
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
	let format =
		ExchangeFormat::from_content_type(request.get_header("content-type"))?;
	let body_bytes = request.body.into_bytes().await?;
	let input: Input = format.deserialize(&body_bytes)?;
	let output: Output = next.call(input).await?;
	let body_bytes = format.serialize(&output)?;
	Response::ok()
		.with_content_type(format.content_type_str())
		.with_body(body_bytes)
		.xok()
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
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

	// -- Request/Response JSON round-trip --

	#[test]
	fn json_request_response() {
		let request =
			Request::with_json("/add", &AddInput { a: 10, b: 20 }).unwrap();

		let response = AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		response.status().xpect_eq(StatusCode::Ok);
		response
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/json");
		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(30);
	}

	// -- JSON is the default when no content-type is specified --

	#[test]
	fn json_default_content_type() {
		let request = Request::post("/add").with_body(r#"{"a":1,"b":2}"#);

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.deserialize_blocking::<u32>()
			.unwrap()
			.xpect_eq(3);
	}

	// -- postcard binary round-trip --

	#[test]
	fn postcard_request_response() {
		let request =
			Request::with_postcard("/add", &AddInput { a: 5, b: 7 }).unwrap();

		let response = AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		response
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/x-postcard");

		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(12);
	}

	// -- unit input with empty body --

	#[test]
	fn unit_input_empty_body() {
		let response = AsyncPlugin::world()
			.spawn(route_tool("unit", func_tool(|_: FuncToolIn<()>| Ok(42i32))))
			.call_blocking::<Request, Response>(Request::get("/"))
			.unwrap();

		let result: i32 = response.deserialize_blocking().unwrap();
		result.xpect_eq(42);
	}

	// -- ExchangeFormat unit tests --

	#[test]
	fn format_from_content_type() {
		ExchangeFormat::from_content_type(None)
			.unwrap()
			.xpect_eq(ExchangeFormat::Json);
		ExchangeFormat::from_content_type(Some("application/json"))
			.unwrap()
			.xpect_eq(ExchangeFormat::Json);
		ExchangeFormat::from_content_type(Some(
			"application/json; charset=utf-8",
		))
		.unwrap()
		.xpect_eq(ExchangeFormat::Json);
		ExchangeFormat::from_content_type(Some("application/x-postcard"))
			.unwrap()
			.xpect_eq(ExchangeFormat::Postcard);

		// unrecognized content-type should error
		ExchangeFormat::from_content_type(Some("text/plain")).xpect_err();
	}

	#[test]
	fn format_roundtrip_json() {
		let input = AddInput { a: 1, b: 2 };
		let bytes = ExchangeFormat::Json.serialize(&input).unwrap();
		let output: AddInput =
			ExchangeFormat::Json.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	#[test]
	fn format_roundtrip_postcard() {
		let input = AddInput { a: 3, b: 4 };
		let bytes = ExchangeFormat::Postcard.serialize(&input).unwrap();
		let output: AddInput =
			ExchangeFormat::Postcard.deserialize(&bytes).unwrap();
		output.xpect_eq(input);
	}

	// -- Response::deserialize round-trip --

	#[test]
	fn response_deserialize_json_roundtrip() {
		let request =
			Request::with_json("/add", &AddInput { a: 10, b: 5 }).unwrap();

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(15);
	}

	#[test]
	fn response_deserialize_postcard_roundtrip() {
		let request =
			Request::with_postcard("/add", &AddInput { a: 3, b: 9 }).unwrap();

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(12);
	}

	// -- with_json_str convenience --

	#[test]
	fn json_str_request() {
		let request = Request::with_json_str("/add", r#"{"a":100,"b":200}"#);

		AsyncPlugin::world()
			.spawn(add_route_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.deserialize_blocking::<i32>()
			.unwrap()
			.xpect_eq(300);
	}

	// -- route tool inserts path partial --

	#[test]
	fn inserts_path_partial() {
		let mut world = RouterPlugin::world();
		world
			.spawn(add_route_tool())
			.get::<PathPattern>()
			.unwrap()
			.annotated_route_path()
			.to_string()
			.xpect_eq("/add");
	}
}
