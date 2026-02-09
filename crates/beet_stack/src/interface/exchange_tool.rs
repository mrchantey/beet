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

/// The serialization format used for exchange tool request/response bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeFormat {
	/// JSON serialization via `serde_json` (`application/json`).
	Json,
	/// Binary serialization via `postcard` (`application/x-postcard`).
	Postcard,
}

impl ExchangeFormat {
	/// Determine the format from a `content-type` header value,
	/// defaulting to JSON if absent, and erroring if unrecognized.
	///
	/// ## Errors
	///
	/// Errors if unrecognized format provided.
	pub fn from_content_type(content_type: Option<&str>) -> Result<Self> {
		match content_type {
			Some(ct) if ct.contains("application/x-postcard") => Self::Postcard,
			Some(ct) if ct.contains("application/json") => Self::Json,
			Some(other) => bevybail!(
				"Unrecognized content-type for exchange tool: {other}. \
				 Supported types: application/json, application/x-postcard."
			),
			None => Self::Json,
		}
		.xok()
	}

	/// The MIME content-type string for this format.
	pub fn content_type_str(&self) -> &'static str {
		match self {
			Self::Json => "application/json",
			Self::Postcard => "application/x-postcard",
		}
	}

	/// Deserialize bytes into `T` using this format.
	///
	/// Empty bytes are treated as JSON `null` for the JSON format,
	/// enabling unit-type inputs on GET requests with no body.
	pub fn deserialize<T: serde::de::DeserializeOwned>(
		&self,
		bytes: &[u8],
	) -> Result<T> {
		match self {
			Self::Json => {
				let slice = if bytes.is_empty() { b"null" } else { bytes };
				serde_json::from_slice(slice).map_err(|err| {
					bevyhow!("Failed to deserialize JSON body: {err}")
				})
			}
			Self::Postcard => postcard::from_bytes(bytes).map_err(|err| {
				bevyhow!("Failed to deserialize postcard body: {err}")
			}),
		}
	}

	/// Serialize `T` into bytes using this format.
	pub fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
		match self {
			Self::Json => serde_json::to_vec(value).map_err(|err| {
				bevyhow!("Failed to serialize JSON response: {err}")
			}),
			Self::Postcard => postcard::to_allocvec(value).map_err(|err| {
				bevyhow!("Failed to serialize postcard response: {err}")
			}),
		}
	}
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
		let request = Request::post("/add")
			.with_header("content-type", "application/json")
			.with_body(r#"{"a":10,"b":20}"#);

		let response = World::new()
			.spawn(add_exchange_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		response.status().xpect_eq(StatusCode::Ok);
		response
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/json");
		// response body is the serialized i32
		let body = response.body.try_into_bytes().unwrap();
		let result: i32 = serde_json::from_slice(&body).unwrap();
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

		let body = response.body.try_into_bytes().unwrap();
		let result: i32 = serde_json::from_slice(&body).unwrap();
		result.xpect_eq(3);
	}

	// -- postcard binary round-trip --

	#[test]
	fn postcard_request_response() {
		let input = AddInput { a: 5, b: 7 };
		let encoded = postcard::to_allocvec(&input).unwrap();

		let request = Request::post("/add")
			.with_body(encoded)
			.with_header("content-type", "application/x-postcard");

		let response = World::new()
			.spawn(add_exchange_tool())
			.call_blocking::<Request, Response>(request)
			.unwrap();

		response
			.get_header("content-type")
			.unwrap()
			.xpect_eq("application/x-postcard");

		let body = response.body.try_into_bytes().unwrap();
		let result: i32 = postcard::from_bytes(&body).unwrap();
		result.xpect_eq(12);
	}

	// -- unit input with empty body --

	#[test]
	fn unit_input_empty_body() {
		let response = World::new()
			.spawn(exchange_tool(|| -> i32 { 42 }))
			.call_blocking::<Request, Response>(Request::get("/"))
			.unwrap();

		let body = response.body.try_into_bytes().unwrap();
		let result: i32 = serde_json::from_slice(&body).unwrap();
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
}
