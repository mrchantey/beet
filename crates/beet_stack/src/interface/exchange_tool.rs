use crate::prelude::*;
use beet_core::prelude::*;

/// Create a tool that can be called with both typed `In`/`Out` and
/// [`Request`]/[`Response`] pairs. When called with `Request`/`Response`,
/// the request body is deserialized to `In` and the `Out` is serialized
/// back into the response body.
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
/// // Create an exchange tool from a typed handler
/// let bundle = exchange_tool(|(a, b): (i32, i32)| -> i32 { a + b });
///
/// // Can be called with typed values
/// let result = World::new()
///     .spawn(bundle)
///     .call_blocking::<(i32, i32), i32>((2, 2))
///     .unwrap();
/// assert_eq!(result, 4);
/// ```
///
/// ## Using with Request/Response exchange pattern
///
/// Exchange tools also support serialized request/response calls,
/// using the `content-type` header for format negotiation:
///
/// ```
/// # use beet_stack::prelude::*;
/// # use beet_core::prelude::*;
/// # use serde::{Serialize, Deserialize};
/// #[derive(Debug, Serialize, Deserialize, Reflect)]
/// struct AddInput { a: i32, b: i32 }
///
/// let request = Request::with_json("/add", &AddInput { a: 10, b: 20 }).unwrap();
/// let response = World::new()
///     .spawn(exchange_tool(|input: AddInput| -> i32 { input.a + input.b }))
///     .call_blocking::<Request, Response>(request)
///     .unwrap();
///
/// 
/// 
/// let result: i32 = async_ext::block_on(response.deserialize()).unwrap();
/// assert_eq!(result, 30);
/// ```
pub fn exchange_tool<H, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
	H::In: serde::de::DeserializeOwned,
	H::Out: serde::Serialize,
{
	(
		ToolMeta::of::<H::In, H::Out>(),
		exchange_tool_handler::<H::In, H::Out>(),
		handler.into_handler(),
	)
}

/// Marker component indicating this tool supports [`Request`]/[`Response`]
/// calls via automatic serialization. Added by [`exchange_tool`].
#[derive(Debug, Component)]
pub struct ExchangeToolMarker;

/// Creates the observer bundle that bridges `Request`/`Response` tool calls
/// to the inner typed `In`/`Out` handler.
///
/// When a [`ToolIn<Request, Response>`] event fires on the entity:
/// 1. The request's `content-type` header determines the [`ExchangeFormat`]
/// 2. The request body is deserialized to `In`
/// 3. A [`ToolIn<In, Out>`] is triggered with a wrapping [`ToolOutHandler`]
/// 4. The wrapping handler serializes `Out` into a [`Response`] body
///    and forwards it to the original `Response` out handler
fn exchange_tool_handler<In, Out>() -> impl Bundle
where
	In: 'static + Send + Sync + serde::de::DeserializeOwned,
	Out: 'static + Send + Sync + serde::Serialize,
{
	(
		ExchangeToolMarker,
		OnSpawn::observe(
			move |mut ev: On<ToolIn<Request, Response>>,
			      mut commands: Commands|
			      -> Result {
				let ev = ev.event_mut();
				let tool = ev.tool();
				let request = ev.take_payload()?;
				let outer_handler = ev.take_out_handler()?;

				// determine serialization format from request content-type
				let format = ExchangeFormat::from_content_type(
					request.get_header("content-type"),
				)?;

				// deserialize request body to In
				let body_bytes =
					request.body.try_into_bytes().ok_or_else(|| {
						bevyhow!(
							"Exchange tool does not support streaming request bodies"
						)
					})?;
				let input: In = format.deserialize(&body_bytes)?;

				// create a wrapping out handler that serializes Out -> Response
				// then forwards to the original Response handler
				let wrapping_handler =
					ToolOutHandler::<Out>::function(move |output: Out| {
						let body_bytes = format.serialize(&output)?;
						let response = Response::ok()
							.with_content_type(format.content_type_str())
							.with_body(body_bytes);
						match outer_handler {
							ToolOutHandler::Channel { sender } => {
								sender.try_send(response).map_err(|err| {
									bevyhow!(
										"Failed to send exchange response: {err:?}"
									)
								})
							}
							ToolOutHandler::Function { handler } => {
								handler(response)
							}
							ToolOutHandler::Observer { .. } => {
								bevybail!(
									"Exchange tool does not support Observer out handlers directly. \
									 Use entity.call() or entity.call_blocking() instead."
								)
							}
						}
					});

				// trigger the inner typed handler via deferred command
				commands.entity(tool).trigger(|entity| {
					ToolIn::new(entity, input, wrapping_handler)
				});
				Ok(())
			},
		),
	)
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::async_ext;
	use beet_core::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;

	#[derive(Debug, Serialize, Deserialize, Reflect, PartialEq)]
	struct AddInput {
		a: i32,
		b: i32,
	}

	fn add_exchange_tool() -> impl Bundle {
		(
			PathPartial::new("add"),
			exchange_tool(|input: AddInput| -> i32 { input.a + input.b }),
		)
	}

	// -- typed calls still work through exchange_tool --

	#[test]
	fn typed_call() {
		World::new()
			.spawn(exchange_tool(|input: AddInput| -> i32 {
				input.a + input.b
			}))
			.call_blocking::<AddInput, i32>(AddInput { a: 3, b: 4 })
			.unwrap()
			.xpect_eq(7);
	}

	// -- Request/Response JSON round-trip --

	#[test]
	fn json_request_response() {
		let request =
			Request::with_json("/add", &AddInput { a: 10, b: 20 }).unwrap();

		let response = World::new()
			.spawn(add_exchange_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		response.status().xpect_eq(StatusCode::Ok);
		response
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/json");
		let result: i32 = async_ext::block_on(response.deserialize()).unwrap();
		result.xpect_eq(30);
	}

	// -- JSON is the default when no content-type is specified --

	#[test]
	fn json_default_content_type() {
		let request = Request::post("/add").with_body(r#"{"a":1,"b":2}"#);

		let response = World::new()
			.spawn(add_exchange_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		let result: i32 = async_ext::block_on(response.deserialize()).unwrap();
		result.xpect_eq(3);
	}

	// -- postcard binary round-trip --

	#[test]
	fn postcard_request_response() {
		let request =
			Request::with_postcard("/add", &AddInput { a: 5, b: 7 }).unwrap();

		let response = World::new()
			.spawn(add_exchange_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		response
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/x-postcard");

		let result: i32 = async_ext::block_on(response.deserialize()).unwrap();
		result.xpect_eq(12);
	}

	// -- unit input with empty body --

	#[test]
	fn unit_input_empty_body() {
		let response = World::new()
			.spawn(exchange_tool(|| -> i32 { 42 }))
			.call_blocking::<Request, Response>(Request::get("/"))
			.unwrap();

		let result: i32 = async_ext::block_on(response.deserialize()).unwrap();
		result.xpect_eq(42);
	}

	// -- calling a non-exchange tool with Request/Response fails --

	#[test]
	#[should_panic = "does not support Request/Response"]
	fn non_exchange_tool_rejects_request() {
		World::new()
			.spawn(tool(|(a, b): (i32, i32)| -> i32 { a + b }))
			.call_blocking::<Request, Response>(Request::get("/"))
			.unwrap();
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

		let response = World::new()
			.spawn(add_exchange_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		let result: i32 = async_ext::block_on(response.deserialize()).unwrap();
		result.xpect_eq(15);
	}

	#[test]
	fn response_deserialize_postcard_roundtrip() {
		let request =
			Request::with_postcard("/add", &AddInput { a: 3, b: 9 }).unwrap();

		let response = World::new()
			.spawn(add_exchange_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		let result: i32 = async_ext::block_on(response.deserialize()).unwrap();
		result.xpect_eq(12);
	}

	// -- with_json_str convenience --

	#[test]
	fn json_str_request() {
		let request = Request::with_json_str("/add", r#"{"a":100,"b":200}"#);

		let response = World::new()
			.spawn(add_exchange_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		let result: i32 = async_ext::block_on(response.deserialize()).unwrap();
		result.xpect_eq(300);
	}
}
